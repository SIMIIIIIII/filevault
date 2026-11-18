resource "helm_release" "filevault" {
    count      = var.environment != "dev" ? 1 : 0
    name       = "filevault"
    chart      = "../charts/filevault"
    namespace  = try(kubernetes_namespace_v1.filevault[0].metadata[0].name, "default")
    create_namespace = false
    wait       = true
    timeout    = 300
    atomic     = true
    cleanup_on_fail = true

    values = [
        fileexists("${path.module}/charts/filevault/values-base.yaml")
        ? file("${path.module}/charts/filevault/values-base.yaml")
        : yamlencode({}),
        fileexists("${path.module}/charts/filevault/values-${var.environment}.yaml")
        ? file("${path.module}/charts/filevault/values-${var.environment}.yaml")
        : yamlencode({}),
        yamlencode({
            image = {
                repository = var.image_repo
                tag        = var.image_tag
            }
            replicaCount = local.replicas
            postgresql = {
                auth = {
                    password = try(random_password.db[0].result, "")
                }
            }
        })
    ]

    depends_on = [
        kubernetes_secret_v1.app[0],
        kubernetes_secret_v1.tls[0],
        kubernetes_persistent_volume_claim_v1.uploads[0],
        helm_release.cert_manager[0]
    ]
}

resource "helm_release" "cert_manager" {
    count           = var.environment != "dev" ? 1 : 0
    name            = "cert-manager"
    repository      = "https://charts.jetstack.io"
    chart           = "cert-manager"
    version         = "1.14.2"
    namespace       = "cert-manager"
    create_namespace = true

    values = [
        yamlencode({
            installCRDs = true
        })
    ]
}

# Output de la release
output "helm_status" {
    value = try(helm_release.filevault[0].status, "")
}

output "filevault_version" {
    value = try(helm_release.filevault[0].version, "")
}