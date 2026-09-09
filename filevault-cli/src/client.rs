use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Deserialize, Debug)]
pub struct File {
    pub id: i64,
    pub name: String,
    pub size: i64,
    pub created_at: String,
    pub file_path: String,
}

#[derive(Deserialize, Debug)]
pub struct Stats {
    pub total_files: i64,
    pub total_bytes: i64,
    pub nb_users: i64,
}

#[derive(Serialize)]
struct LoginRequest<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

pub async fn list_files(client: &reqwest::Client, base_url: &str) -> anyhow::Result<Vec<File>> {
    let reponse = client
        .get(format!("{base_url}/files"))
        .send()
        .await?
        .error_for_status()?;

    let files = reponse.json::<Vec<File>>().await?;

    Ok(files)
}

pub async fn statistics(client: &reqwest::Client, base_url: &str) -> anyhow::Result<Stats> {
    let stats = client
        .get(format!("{base_url}/stats"))
        .send()
        .await?
        .error_for_status()?
        .json::<Stats>()
        .await?;

    Ok(stats)
}

pub async fn login(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    pwd: &str,
) -> anyhow::Result<String> {
    let reponse = client
        .post(format!("{base_url}/auth/login"))
        .json(&LoginRequest {
            email,
            password: pwd,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<LoginResponse>()
        .await?;

    Ok(reponse.token)
}

pub async fn list_files_authentified(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
) -> anyhow::Result<Vec<File>> {
    let files = client
        .get(format!("{base_url}/files"))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<File>>()
        .await?;
    Ok(files)
}

pub async fn upload_file(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    local_path: &std::path::Path,
) -> anyhow::Result<File> {
    let file_name = local_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("chemin invalide"))?
        .to_string_lossy()
        .into_owned();

    let size = fs::metadata(local_path).await?.len();
    let barre = ProgressBar::new(size);

    barre.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes}",
        )?
        .progress_chars("=>-"),
    );

    let file = fs::File::open(local_path).await?;
    let barre_clone = barre.clone();
    let stream = tokio_util::io::ReaderStream::new(file).map(move |chunk| {
        if let Ok(bytes) = &chunk {
            barre_clone.inc(bytes.len() as u64);
        }
        chunk
    });

    let part = multipart::Part::stream_with_length(reqwest::Body::wrap_stream(stream), size)
        .file_name(file_name);

    let form = multipart::Form::new().part("file", part);

    let created_file = client
        .post(format!("{base_url}/files"))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?
        .json::<File>()
        .await?;

    barre.finish_with_message("upload terminé");

    Ok(created_file)
}

pub async fn download_file(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    id: i64,
    exit_path: &std::path::Path,
) -> anyhow::Result<()> {
    let already_received = match fs::metadata(exit_path).await {
        Ok(meta) => meta.len(),
        Err(_) => 0,
    };

    let mut request = client
        .get(format!("{base_url}/files/{id}"))
        .bearer_auth(token);

    if already_received > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={already_received}-"));
    }

    let reponse = request.send().await?.error_for_status()?;
    let total_size = reponse.content_length().unwrap_or(0) + already_received;

    let barre = ProgressBar::new(total_size);
    barre.set_position(already_received);
    barre.set_style(ProgressStyle::with_template(
        "{spinner:.green} [{bar:40.green/blue}] {bytes}/{total_bytes}",
    )?);

    let mut fichier = tokio::fs::OpenOptions::new()
        .create(true)
        .append(already_received > 0)
        .write(true)
        .open(exit_path)
        .await?;

    let mut flux = reponse.bytes_stream();
    while let Some(morceau) = flux.next().await {
        let morceau = morceau?;
        fichier.write_all(&morceau).await?;
        barre.inc(morceau.len() as u64);
    }

    barre.finish_with_message("Downloading done!");
    Ok(())
}

#[derive(Deserialize)]
struct ErreurServeur {
    error: String,
}

pub async fn verify_response(reponse: reqwest::Response) -> anyhow::Result<reqwest::Response> {
    if reponse.status().is_success() {
        return Ok(reponse);
    }

    let statut = reponse.status();
    let corps = reponse.text().await.unwrap_or_default();
    let message = serde_json::from_str::<ErreurServeur>(&corps)
        .map(|e| e.error)
        .unwrap_or(corps);

    anyhow::bail!("[SERVER] {statut}: {message}")
}
