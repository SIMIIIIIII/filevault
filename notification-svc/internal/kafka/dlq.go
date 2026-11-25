package kafka

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/twmb/franz-go/pkg/kgo"
)

type DLQMessage struct {
	OriginalTopic     string `json:"original_topic"`
	OriginalPartition int32  `json:"original_partition"`
	OriginalOffset    int64  `json:"original_offset"`
	OriginalKey       string `json:"original_key"`
	OriginalValue     string `json:"original_value"`
	ErrorMessage      string `json:"error_message"`
	AttemptCount      int    `json:"attempt_count"`
	FailedAt          string `json:"failed_at"`
	ServiceName       string `json:"service_name"`
}

const MaxRetries = 3

// ProcessWithRetry traite un record avec retry et DLQ
func ProcessWithRetry(
	ctx context.Context,
	rec *kgo.Record,
	handler func(context.Context, *kgo.Record) error,
	dlqClient *kgo.Client,
	dlqTopic string,
	svcName string,
) error {
	var lastErr error
	for attempt := 1; attempt <= MaxRetries; attempt++ {
		if err := handler(ctx, rec); err == nil {
			return nil // Succès
		} else {
			lastErr = err
			fmt.Printf("Tentative %d/%d échouée offset=%d: %v\n",
				attempt, MaxRetries, rec.Offset, err)
			if attempt < MaxRetries {
				// Backoff exponentiel : 100ms, 200ms, 400ms
				backoff := time.Duration(100*(1<<(attempt-1))) * time.Millisecond
				select {
				case <-time.After(backoff):
				case <-ctx.Done():
					return ctx.Err()
				}
			}
		}
	}

	// Tous les retries épuisés → envoyer en DLQ
	dlqMsg := DLQMessage{
		OriginalTopic:     rec.Topic,
		OriginalPartition: rec.Partition,
		OriginalOffset:    rec.Offset,
		OriginalKey:       string(rec.Key),
		OriginalValue:     string(rec.Value),
		ErrorMessage:      lastErr.Error(),
		AttemptCount:      MaxRetries,
		FailedAt:          time.Now().UTC().Format(time.RFC3339),
		ServiceName:       svcName,
	}

	dlqPayload, _ := json.Marshal(dlqMsg)
	dlqRecord := &kgo.Record{
		Topic: dlqTopic,
		Key:   rec.Key, // Garder la clé originale
		Value: dlqPayload,
	}

	if err := dlqClient.ProduceSync(ctx, dlqRecord).FirstErr(); err != nil {
		// La DLQ elle-même a échoué — logguer l"erreur critique
		fmt.Printf("CRITIQUE: DLQ push échoué offset=%d: %v\n", rec.Offset, err)
		return fmt.Errorf("DLQ push: %w", err)
	}

	fmt.Printf("Message envoyé en DLQ topic=%s offset=%d\n",
		dlqTopic, rec.Offset)
	return nil // Traité (vers DLQ) — committer l"offset
}
