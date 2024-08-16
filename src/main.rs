use std::sync::Arc;
use anyhow::Result;
use llmchain::{GithubPRSummary, LLM, Summarize};
use octocrab::Octocrab;
use llmchain::{Document, Documents};
use llmchain::OpenAI;
use llmchain::OpenAIGenerateModel;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let openai_api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");
    let github_token = env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN must be set");
    let owner = env::var("GITHUB_OWNER")
        .expect("GITHUB_OWNER must be set");
    let repo = env::var("GITHUB_REPO")
        .expect("GITHUB_REPO must be set");
    let pr_number: u64 = env::var("GITHUB_PR_NUMBER")
        .expect("GITHUB_PR_NUMBER must be set")
        .parse()
        .expect("GITHUB_PR_NUMBER must be a valid number");

    let llm: Arc<dyn LLM> = OpenAI::create(&openai_api_key)
        .with_generate_model(OpenAIGenerateModel::Gpt35) //Gpt35 or Gpt4o 
        .with_temperature(0.7);

    let octocrab = Octocrab::builder()
        .personal_token(github_token)
        .build()?;

    let summarizer = GithubPRSummary::create(llm);

    let pr = octocrab
        .pulls(&owner, &repo)
        .get(pr_number)
        .await?;

    let diff = octocrab
        .pulls(&owner, &repo)
        .get_diff(pr_number)
        .await?;

    let document = Document {
        path: "pr_diff".to_string(),
        content: diff.clone(),
        content_md5: Default::default(),
    };

    let documents = Documents::from(vec![document]);

    summarizer.add_documents(&documents).await?;
    let summary = summarizer.final_summary().await?;

    let output = format!(
        "PR Summary for {}/{} #{}\n\nTitle: {}\n\nSummary:\n{}\n\nTotal tokens used: {}\n\nPatch (Changes):\n{}",
        owner,
        repo,
        pr_number,
        pr.title.unwrap_or_default(),
        summary,
        summarizer.tokens(),
        diff
    );

    let file_path = Path::new("./demo-RAG-embeddings/pr_summary.txt");

    let mut file = File::create(file_path)?;
    file.write_all(output.as_bytes())?;

    println!("PR summary has been saved to: {:?}", file_path);

    Ok(())
}