use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "filevault-cli", version, about = "Client HTTP foor FileVault")]
pub struct Cli {
    #[arg(long, global = true, default_value = "http://localhost:8081")]
    pub server: String,
    #[command(subcommand)]
    pub commande: Commande,
}

#[derive(Subcommand)]
pub enum Commande {
    Login {
    #[arg(long)]
        email: String,
        #[arg(long)]
        mot_de_passe: String,
    },

    /// Envoyer un fichier vers le serveur
    Upload {
        fichier: PathBuf,
    },

    /// Télécharger un fichier depuis le serveur
    Download {
        id: i64,
        #[arg(long, short)]
        output: PathBuf,
    },

    /// Lister les fichiers stockés
    List,
    
    /// Afficher les statistiques du serveur
    Stats,
}