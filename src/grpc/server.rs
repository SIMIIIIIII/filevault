use std::path::Path;
use std::pin::Pin;
use std::convert::TryFrom;

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tonic::{Request, Response, Status, Streaming};
use sha2::Digest;

pub mod filevault {
    tonic::include_proto!("filevault");
}

use filevault::file_vault_server::FileVault;
use filevault::*;

use crate::api::auth::generate_token;
use crate::db::{check_user, delete_file, get_all_files, get_file, insert_file, new_user, update_nb_download};
use crate::grpc::auth::authenticate_request;

pub struct FileVaultService {
    pub pool: sqlx::PgPool,
    pub jwt_secret: String,
    pub max_upload_bytes: u64
}

pub struct MyFile {
    pub file : fs::File,
    pub file_path : String
}

#[tonic::async_trait]
impl FileVault for FileVaultService {
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, Status> {
        let claims = authenticate_request(&request, &self.jwt_secret)?;
        let user_id = claims.sub;
        
        let req = request.into_inner();
        tracing::info!(user_id = user_id, "Files listing");
        

        let files = 
        get_all_files(
            self.pool.clone(),
            user_id as i32,
            req.limit as i64,
            req.page as i64
        )
        .await
        .map_err(|error| Status::internal(error.to_string()))?;

        let total = i32::try_from(files.len())
            .map_err(|_| Status::internal("too many files to return"))?;

        let files = files
            .into_iter()
            .map(|file| File {
                id: i64::from(file.id),
                user_id: i64::from(file.user_id.unwrap_or_default()),
                name: file.name,
                size: file.size,
                sha256: file.sha256,
                file_path: file.file_path,
                created_at: file.created_at.timestamp(),
                nb_downloads: file.nb_downloads,
            })
            .collect();

        Ok(Response::new(ListResponse { files, total }))
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let claims = authenticate_request(&request, &self.jwt_secret)?;
        let user_id = claims.sub;

        let file_id = i32::try_from(request.into_inner().id)
            .map_err(|_| Status::invalid_argument("file id is out of range"))?;

        tracing::info!(user_id = user_id, "Files deleting");

        delete_file(self.pool.clone(), file_id, user_id as i32)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(DeleteResponse { succes: true }))
    }

    async fn upload(
        &self,
        request: Request<Streaming<UploadChunk>>,
    ) -> Result<Response<UploadResponse>, Status> {
        let claims = authenticate_request(&request, &self.jwt_secret)?;
        let user_id = claims.sub;
        let mut stream = request.into_inner();
        let mut metadata: Option<UploadMetadata> = None;
        let mut total_bytes = 0u64;
        let mut hasher = sha2::Sha256::new();

        let mut my_file = MyFile {
            file :
                fs::File::create_new("test")
                .await
                .map_err(|e| Status::internal(e.to_string()))?,
            file_path: "test".to_string()
        };

        while let Some(chunk) = stream.message().await? {
            match chunk.payload {
                Some(upload_chunk::Payload::Metadata(m)) => {
                    tracing::info!(name = %m.name, size = m.size, "Upload started");
                    metadata = Some(m.clone());
                    
                    let safe_name =
                        Path::new(&m.name)
                        .file_name()
                        .ok_or(Status::internal("Can't créate file"))?
                        .to_string_lossy()
                        .into_owned();

                    let file_path =
                        Path::new("server_files")
                        .join(&safe_name);

                        
                    let _ = fs::remove_file(my_file.file_path).await;
                    let file_disc = 
                        fs::File::create(&file_path)
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?;

                    my_file = MyFile {
                        file: file_disc,
                        file_path: file_path.to_string_lossy().to_string()
                    }
                }

                Some(upload_chunk::Payload::Data(data)) => {
                    if metadata.is_none() {
                        let _ = fs::remove_file(my_file.file_path).await;
                        return Err(Status::data_loss("Metada not found"))
                    }
                    use sha2::Digest;
                    hasher.update(&data);
                    total_bytes += data.len() as u64;

                    if total_bytes > self.max_upload_bytes {
                        let _ = fs::remove_file(my_file.file_path).await;
                        return Err(Status::out_of_range("Data too long"))
                    }

                    my_file.file.write_all(&data)
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?;
                }
                None => {}
            }
        }

        let metadata = metadata
            .ok_or_else(|| Status::data_loss("Metadata manquante"))?;

        
        let sha256 = hex::encode(hasher.finalize());
        if !metadata.sha256.is_empty() && metadata.sha256 != sha256 {
            let _ = fs::remove_file(&my_file.file_path).await;
            return Err(Status::data_loss("SHA-256 invalide"));
        }

        if metadata.size != total_bytes as i64 {
            let _ = fs::remove_file(&my_file.file_path).await;
            return Err(Status::data_loss("invalid size"));
        }

        let res = insert_file(
            self.pool.clone(),
            metadata.name,
            metadata.size,
            sha256.clone(),
            my_file.file_path,
            user_id as i32
        ).await
        .map_err(|error| Status::internal(error.to_string()))?;

        tracing::info!(octets = total_bytes, sha256 = sha256, "Upload done");

        Ok(Response::new(res))

    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Login");

        let user = check_user(self.pool.clone(), req.email, req.password)
            .await
            .map_err(|error| Status::internal(error.to_string()))?;

        let token = generate_token(user.id as i64, &self.jwt_secret.to_string())
            .map_err(|error| Status::internal(error.to_string()))?;

        Ok(Response::new(LoginResponse {token: token}))
    }

    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Registering");

        let hash_pwd = bcrypt::hash(req.password, bcrypt::DEFAULT_COST)
        .map_err(|error| Status::internal(error.to_string()))?;

        new_user(self.pool.clone(), req.username, req.fullname, req.email, hash_pwd)
            .await
            .map_err(|error| Status::internal(error.to_string()))?;

        Ok(Response::new(RegisterResponse { status: 2}))
    }

    type DownloadStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<DataChunk, Status>> + Send>>;

    async fn download(
        &self,
        request: Request<DownloadRequest>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        let claims = authenticate_request(&request, &self.jwt_secret)?;
        let user_id = claims.sub;
        
        let req = request.into_inner();
        tracing::info!(user_id = user_id, "Files listing");

        let file_id = req.file_id;

        let file_info = get_file(self.pool.clone(), user_id as i32, file_id as i32)
        .await
        .map_err(|error| Status::internal(error.to_string()))?;
        
        update_nb_download(self.pool.clone(), user_id as i32, file_id as i32)
        .await
        .map_err(|error| Status::internal(error.to_string()))?;

        let stream = async_stream::try_stream! {
            let mut file = tokio::fs::File::open(
                file_info.file_path
            ).await.map_err(|e| Status::not_found(e.to_string()))?;

            let mut buf = vec![0u8; 64 * 1024];
            let mut offset = 0i64;

            loop {
                let n = file.read(&mut buf).await
                    .map_err(|e| Status::internal(e.to_string()))?;
                
                if n == 0 { break; }
                yield DataChunk {
                    data: buf[..n].to_vec(),
                    offset,
                    last: false,
                };
                offset += n as i64;
            }

            yield DataChunk { data: vec![], offset, last: true };
            
        };

        
        Ok(Response::new(Box::pin(stream)))
    }
}