package grpc

import (
	"context"
	"fmt"
	"io"

	pb "github.com/SIMIIIII/api-gateway/filevault"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/metadata"
)

type ClientFileVault struct {
	conn   *grpc.ClientConn
	client pb.FileVaultClient
	token  string
}

// Connexion avec TLS vers le serveur Rust
func Connect(addr string, caFile string) (*ClientFileVault, error) {
	creds, err := credentials.NewClientTLSFromFile(caFile, "localhost")
	if err != nil {
		return nil, fmt.Errorf("charger CA: %w", err)
	}

	conn, err := grpc.NewClient(
		addr,
		grpc.WithTransportCredentials(creds),
	)

	if err != nil {
		return nil, fmt.Errorf("connexion gRPC: %w", err)
	}

	return &ClientFileVault{
		conn:   conn,
		client: pb.NewFileVaultClient(conn),
	}, nil
}

func (c *ClientFileVault) Fermer() error {
	return c.conn.Close()
}

// GetToken returns the stored authentication token
func (c *ClientFileVault) GetToken() (string, error) {
	if c.token == "" {
		return "", fmt.Errorf("no token stored - please login first")
	}
	return c.token, nil
}

// Appel unary : lister les fichiers
func (c *ClientFileVault) Lister(ctx context.Context, userID int64) ([]*pb.File, error) {
	resp, err := c.client.List(ctx, &pb.ListRequest{
		Page:  1,
		Limit: 50,
	})

	if err != nil {
		return nil, fmt.Errorf("lister: %w", err)
	}

	return resp.Files, nil
}

// connect au serveur
func (c *ClientFileVault) Login(ctx context.Context, email string, password string) error {
	token, err := c.client.Login(ctx, &pb.LoginRequest{
		Email:    email,
		Password: password,
	})

	if err != nil {
		return err
	}

	c.token = token.Token

	return nil
}

func (c *ClientFileVault) List(
	ctx context.Context, page int32, limit int32) (pb.ListResponse, error) {
	// Add token to request metadata with Bearer format
	ctx = metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+c.token)

	listFiles, err := c.client.List(ctx, &pb.ListRequest{
		Page:  page,
		Limit: limit,
	})

	if err != nil {
		return pb.ListResponse{}, err
	}
	return *listFiles, nil
}

// Upload en client-streaming vers le serveur Rust
func (c *ClientFileVault) Upload(
	ctx context.Context,
	nom string,
	data []byte) (*pb.UploadResponse, error) {
	// Add token to request metadata with Bearer format
	ctx = metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+c.token)

	stream, err := c.client.Upload(ctx)
	if err != nil {
		return nil, err
	}

	// 1. Envoyer les métadonnées
	err = stream.Send(&pb.UploadChunk{
		Payload: &pb.UploadChunk_Metadata{
			Metadata: &pb.UploadMetadata{
				Name: nom,
				Size: int64(len(data)),
			},
		},
	})
	if err != nil {
		return nil, err
	}

	// 2. Envoyer les données par chunks de 64KB
	chunkSize := 64 * 1024
	for i := 0; i < len(data); i += chunkSize {
		end := i + chunkSize
		if end > len(data) {
			end = len(data)
		}
		err = stream.Send(&pb.UploadChunk{
			Payload: &pb.UploadChunk_Data{
				Data: data[i:end],
			},
		})
		if err != nil {
			return nil, err
		}
	}

	// 3. Fermer le stream et récupérer la réponse
	return stream.CloseAndRecv()
}

// Register creates a new user account
func (c *ClientFileVault) Register(ctx context.Context, username string, fullname string, email string, password string) error {
	resp, err := c.client.Register(ctx, &pb.RegisterRequest{
		Username: username,
		Fullname: fullname,
		Email:    email,
		Password: password,
	})

	if err != nil {
		return err
	}

	if resp.Status != 2 {
		return fmt.Errorf("registration failed with status: %d", resp.Status)
	}

	return nil
}

// Delete removes a file from the server
func (c *ClientFileVault) Delete(ctx context.Context, fileID int64) error {
	// Add token to request metadata with Bearer format
	ctx = metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+c.token)

	resp, err := c.client.Delete(ctx, &pb.DeleteRequest{
		Id: fileID,
	})

	if err != nil {
		return err
	}

	if !resp.Succes {
		return fmt.Errorf("deletion failed")
	}

	return nil
}

// Download downloads a file from the server (server-streaming)
func (c *ClientFileVault) Download(ctx context.Context, fileID int64) ([]byte, error) {
	// Add token to request metadata with Bearer format
	ctx = metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+c.token)

	stream, err := c.client.Download(ctx, &pb.DownloadRequest{
		FileId: fileID,
	})

	if err != nil {
		return nil, err
	}

	var data []byte

	for {
		chunk, err := stream.Recv()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}

		data = append(data, chunk.Data...)
	}

	return data, nil
}
