package kafka

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/twmb/franz-go/pkg/kgo"
)

// FileEvent correspond au type publié par Rust
type FileEvent struct {
	Event      string `json:"event"`
	FileID     int64  `json:"file_id"`
	UserID     int64  `json:"user_id"`
	Nom        string `json:"nom"`
	Taille     int64  `json:"taille"`
	SHA256     string `json:"sha256"`
	UploadedAt string `json:"uploaded_at"`
}

type NotificationConsumer struct {
	client  *kgo.Client
	handler func(ctx context.Context, event FileEvent) error
}

func NewConsumer(brokers []string, groupID string,
	handler func(context.Context, FileEvent) error) (*NotificationConsumer, error) {
	client, err := kgo.NewClient(
		kgo.SeedBrokers(brokers...),
		kgo.ConsumerGroup(groupID),
		kgo.ConsumeTopics("file-events"),
		kgo.Balancers(kgo.CooperativeStickyBalancer()),    // Stratégie de partition assignment (réduit le temps de rebalancing)
		kgo.DisableAutoCommit(),                           // pas d"auto-commit, On committera manuellement après traitement
		kgo.ConsumeResetOffset(kgo.NewOffset().AtStart()), // Démarrer depuis le début si pas d"offset connu
		kgo.SessionTimeout(30*time.Second),
		kgo.HeartbeatInterval(3*time.Second),
	)
	if err != nil {
		return nil, fmt.Errorf("créer client Kafka: %w", err)
	}
	return &NotificationConsumer{client: client, handler: handler}, nil
}

// Démarrer la boucle de consommation
func (c *NotificationConsumer) Start(ctx context.Context) error {
	defer c.client.Close()
	for {
		// Poll : récupérer les prochains messages (bloque jusqu"à en avoir)
		fetches := c.client.PollFetches(ctx)
		if ctx.Err() != nil {
			return ctx.Err()
		}

		// Traiter les erreurs de fetch
		if errs := fetches.Errors(); len(errs) > 0 {
			for _, err := range errs {
				// Erreur de partition temporaire : logguer et continuer
				fmt.Printf("Erreur fetch partition %d: %v\n",
					err.Partition, err.Err)
			}
			continue
		}

		// Itérer sur les records reçus
		fetches.EachRecord(func(rec *kgo.Record) {
			if err := c.traiterRecord(ctx, rec); err != nil {
				fmt.Printf("Erreur traitement offset %d: %v\n",
					rec.Offset, err)
				// Ne pas committer cet offset — le message sera retraité
				return
			}
			// Marquer pour commit (pas encore commité !)
			c.client.MarkCommitRecords(rec)
		})

		// Committer les offsets marqués
		// CommitUncommittedOffsets est synchrone (attend l"ACK du broker)
		if err := c.client.CommitUncommittedOffsets(ctx); err != nil {
			fmt.Printf("Erreur commit offsets: %v\n", err)
		}
	}
}

func (c *NotificationConsumer) traiterRecord(ctx context.Context, rec *kgo.Record) error {
	var event FileEvent
	if err := json.Unmarshal(rec.Value, &event); err != nil {
		// Message malformé : logguer et committer quand même (DLQ)
		fmt.Printf("Message malformé offset %d: %v\n", rec.Offset, err)
		return nil // Ne pas bloquer sur un message corrompu
	}
	fmt.Printf("Traitement event=%s user=%d file=%d offset=%d\n",
		event.Event, event.UserID, event.FileID, rec.Offset)
	// Appeler le handler métier
	return c.handler(ctx, event)
}
