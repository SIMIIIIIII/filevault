package websocket

import (
	"sync"
)

// Hub gère les connections WebSocket des utilisateurs
type Hub struct {
	clients    map[int64]*Client // user_id -> Client
	broadcast  chan Message
	register   chan *Client
	unregister chan *Client
	mu         sync.RWMutex
}

// Message représente un message à envoyer à un utilisateur
type Message struct {
	UserID  int64
	Content string
}

// Client représente une connexion WebSocket
type Client struct {
	UserID int64
	Send   chan string
}

// NewHub crée un nouveau hub WebSocket
func NewHub() *Hub {
	return &Hub{
		clients:    make(map[int64]*Client),
		broadcast:  make(chan Message, 256),
		register:   make(chan *Client),
		unregister: make(chan *Client),
	}
}

// Run démarre la boucle du hub
func (h *Hub) Run() {
	for {
		select {
		case client := <-h.register:
			h.mu.Lock()
			h.clients[client.UserID] = client
			h.mu.Unlock()

		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client.UserID]; ok {
				delete(h.clients, client.UserID)
				close(client.Send)
			}
			h.mu.Unlock()

		case message := <-h.broadcast:
			h.mu.RLock()
			if client, ok := h.clients[message.UserID]; ok {
				select {
				case client.Send <- message.Content:
				default:
					// Channel plein, ignorer
				}
			}
			h.mu.RUnlock()
		}
	}
}

// EnvoyerAUser envoie un message à un utilisateur spécifique
func (h *Hub) EnvoyerAUser(userID int64, content string) error {
	h.broadcast <- Message{
		UserID:  userID,
		Content: content,
	}
	return nil
}

// Register enregistre un client
func (h *Hub) Register(client *Client) {
	h.register <- client
}

// Unregister désenregistre un client
func (h *Hub) Unregister(client *Client) {
	h.unregister <- client
}
