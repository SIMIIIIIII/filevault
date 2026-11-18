locals {
  # Nom commun pour toutes les ressources
  app_name = "filevault-${var.environment}"

  # Labels K8s communs
  common_labels = {
    "app.kubernetes.io/name"       = "filevault"
    "app.kubernetes.io/instance"   = var.environment
    "app.kubernetes.io/managed-by" = "terraform"
    "environment"                  = var.environment
  }
  # Décision de nombre de replicas
  replicas = var.environment == "prod" ? 3 : 1
}