mod cli;
mod client;
mod config;
use clap::Parser;
use cli::{Cli, Commande};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let http = reqwest::Client::new();

    match args.commande {
        Commande::Login { email, mot_de_passe } => {
            let token = client::login(&http, &args.server, &email, &mot_de_passe).await?;
            config::save_token(&token)?;
            println!("Connected with success, token saved.");
        }

        Commande::Upload { fichier } => {
            let token = config::load_token()
                .ok_or_else(|| anyhow::anyhow!("First connect with `login`"))?;

            let resultat = client::upload_file(
                &http,
                &args.server,
                &token, &fichier
            ).await?;

            println!("File send : id={} name={}", resultat.id, resultat.name);
        }

        Commande::Download { id, output } => {
            let token = config::load_token()
                .ok_or_else(|| anyhow::anyhow!("First connect with `login`"))?;

            client::download_file(&http, &args.server, &token, id, &output).await?;
            println!("Download to {}", output.display());
        }

        Commande::List => {
            let token = config::load_token()
                .ok_or_else(|| anyhow::anyhow!("First connect with `login`"))?;

            let files = client::list_files_authentified(
                &http,
                &args.server,
                &token
            ).await?;

            for f in files {
                println!("{:>6} {:<30} {} octets", f.id, f.name, f.size);
            }
        }

        Commande::Stats => {
            let stats = client::statistics(&http, &args.server).await?;
            println!(
                "{} files, {} vytes, {} users",
                stats.total_files, stats.total_bytes, stats.nb_users
            );
        }
    }
    
    Ok(())
}