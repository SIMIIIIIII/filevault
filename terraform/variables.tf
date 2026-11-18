variable "environment" {
  description = "Environnement de déploiement (dev, staging, prod)"
  type        = string
  default     = "dev"

  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Valeur autorisée : dev, staging ou prod."
  }
}

variable "image_repo" {
  description = "Repository Docker pour l'image FileVault"
  type        = string
  default     = "filevault"
}

variable "image_tag" {
  description = "Tag de l image Docker FileVault"
  type        = string
  # Pas de default → obligatoire, erreur si non fourni
}

variable "domain" {
  description = "Domaine pour accéder à FileVault"
  type        = string
  default     = "filevault.local"
}

variable "db_password" {
  description = "Mot de passe PostgreSQL"
  type        = string
  sensitive   = true # Masqué dans les logs et le plan !
}

variable "replicas" {
  description = "Nombre de replicas API"
  type        = number
  default     = 2
}

variable "tags" {
  description = "Tags appliqués à toutes les ressources"
  type        = map(string)
  default = {
    project    = "filevault"
    managed-by = "terraform"
  }
}

variable "enabled_features" {
  description = "Fonctionnalités activées"
  type        = list(string)
  default     = ["api", "metrics"]
}