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
  # Base64 encode du certificat CA - only load for non-dev environments
  ca_cert_b64 = var.environment != "dev" ? filebase64("${path.module}/../pki/ca/ca.crt") : ""
}