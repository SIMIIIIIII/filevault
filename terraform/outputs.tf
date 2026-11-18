output "filevault_url" {
  description = "URL publique de FileVault"
  value       = "https://${var.domain}"
}

output "kubeconfig_command" {
  description = "Commande pour configurer kubectl"
  value       = "export KUBECONFIG=/etc/rancher/k3s/k3s.yaml"
}

output "postgres_password" {
  description = "Mot de passe PostgreSQL généré"
  value       = try(random_password.db[0].result, null)
  sensitive   = true # Masqué dans terraform output
}