package cmd

import (
	"context"
	"fmt"
	"log"
	"os/signal"
	"syscall"

	"github.com/SIMIIIII/notification-svc/internal/kafka"
	"github.com/SIMIIIII/notification-svc/internal/websocket"
)

func main() {
	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGTERM, syscall.SIGINT)
	defer cancel()

	// Initialiser le hub WebSocket
	wsHub := websocket.NewHub()
	go wsHub.Run()

	// Handler : envoyer une notification WebSocket
	handler := func(ctx context.Context, event kafka.FileEvent) error {
		switch event.Event {
		case "uploaded":
			msg := fmt.Sprintf("Fichier '%s' uploadé avec succès !", event.Nom)
			return wsHub.EnvoyerAUser(event.UserID, msg)
		case "deleted":
			msg := fmt.Sprintf("Fichier supprimé (ID: %d)", event.FileID)
			return wsHub.EnvoyerAUser(event.UserID, msg)
		case "quota_atteint":
			return wsHub.EnvoyerAUser(event.UserID,
				"Quota de stockage atteint — supprimez des fichiers")
		}
		return nil
	}

	consumer, err := kafka.NewConsumer(
		[]string{"kafka:9092"},
		"notifications", // Consumer group ID
		handler,
	)

	if err != nil {
		log.Fatal(err)
	}

	log.Println("Consumer démarré — group: notifications")
	if err := consumer.Start(ctx); err != nil && err != context.Canceled {
		log.Fatalf("Consumer error: %v", err)
	}
	log.Println("Consumer arrêté proprement")
}
