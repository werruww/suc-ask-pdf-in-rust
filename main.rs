/// Main: The entry point of our AI-powered circus.
// ... (التعليقات كما هي)
mod agent;
mod config;
mod conversation;
mod model;
mod role;
mod utils;
mod tools;

use agent::{Agent, AcademicAgent, ToolingAgent};
use model::ModelManager;
use role::{Audience, Preset, Role};
use serde_json::Value;
use std::fs;
use std::io::{self, Write}; // io::Write needed for flush
use utils::PdfReader;
use env_logger;
use log::info;

/// Reads input from a file, because apparently typing is too mainstream.
// ... (بقية الدالة كما هي)
#[allow(dead_code)]
fn read_input_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if file_path.to_lowercase().ends_with(".pdf") {
        Ok(PdfReader::read_pdf_file(file_path)?)
    } else {
        Ok(fs::read_to_string(file_path)?)
    }
}

/// The main function that makes everything work (or at least tries to).
// ... (بقية التعليقات كما هي)
#[tokio::main] // This attribute macro makes the async main function work with Tokio
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = config::Config::load()?;
    info!("Loaded configuration with search provider: {}", config.search.provider);

    // Initialize model manager
    let model_manager = ModelManager::new(config.ollama.base_url.clone())?;
    let model_name = "llama2";

    // List available models
    info!("Listing available models...");
    let models = model_manager.list_models().await?;
    for model in models.models {
        println!(
            "Model: {}, Size: {} bytes, Modified: {}",
            model.name, model.size, model.modified_at
        );
    }

    // Check if default model exists and pull it if needed
    if !model_manager.model_exists(&model_name).await? {
        println!("Pulling model {} (this might be {} or {} if not fully qualified)...", model_name, model_name, format!("{}:latest", model_name));
        let mut stream = model_manager.pull_model(&model_name).await?;
        while let Some(chunk) = stream.chunk().await? {
            if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                let v: Value = serde_json::from_str(&text)?;
                if let Some(status) = v["status"].as_str() {
                    print!("Status: {}\r", status);
                    io::stdout().flush()?;
                }
            }
        }
        println!("\nModel pulled successfully!");
    } else {
        info!("Model {} already exists.", model_name);
    }

    // Initialize agents
    let mut academic_agent = AcademicAgent::new(config.clone())?;
    let mut tooling_agent = ToolingAgent::new(config)?;

    // --- Academic Agent Interaction ---
    println!("\n--- Academic Agent ---");
    let conversation_id_academic = academic_agent.start_conversation(&model_name);
    println!("Academic Agent Conversation ID: {}", conversation_id_academic);

    println!("Do you want to provide a PDF file path or type a question directly?");
    println!("1. PDF file path");
    println!("2. Type a question");
    print!("Enter your choice (1 or 2): ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let choice = choice.trim();

    // *** التصحيح هنا ***
    let mut input_content_academic: String; // جعل المتغير قابلاً للتغيير

    if choice == "1" {
        print!("Enter the path to your PDF file: ");
        io::stdout().flush()?;
        let mut file_path_input = String::new();
        io::stdin().read_line(&mut file_path_input)?;
        let file_path = file_path_input.trim();

        if !file_path.is_empty() {
            info!("Reading PDF file: {}", file_path);
            match read_input_file(file_path) {
                Ok(pdf_text) => {
                    print!("Enter your question about the PDF: ");
                    io::stdout().flush()?;
                    let mut pdf_question = String::new();
                    io::stdin().read_line(&mut pdf_question)?;
                    input_content_academic = format!(
                        "Based on the following document content (first 2000 chars):\n---\n{}...\n---\nAnswer this question: {}",
                        pdf_text.chars().take(2000).collect::<String>(),
                        pdf_question.trim()
                    );
                    info!("Processing PDF content and your question...");
                }
                Err(e) => {
                    eprintln!("Error reading PDF file: {}. Using a default question.", e);
                    input_content_academic = "What is game theory in one sentence?".to_string();
                }
            }
        } else {
            println!("No PDF path provided. Using a default question.");
            input_content_academic = "What is game theory in one sentence?".to_string();
        }
    } else if choice == "2" {
        print!("Enter your question for the Academic Agent: ");
        io::stdout().flush()?;
        let mut user_question = String::new();
        io::stdin().read_line(&mut user_question)?;
        input_content_academic = user_question.trim().to_string(); // التعيين الأول
        if input_content_academic.is_empty() {
            println!("No question entered. Using a default question.");
            input_content_academic = "What is game theory in one sentence?".to_string(); // التعيين الثاني (الآن مسموح به)
        }
    } else {
        println!("Invalid choice. Using a default question.");
        input_content_academic = "What is game theory in one sentence?".to_string();
    }

    let role_academic = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

    let mut response_academic = academic_agent
        .chat_with_history(
            &conversation_id_academic,
            &input_content_academic,
            Some(role_academic),
        )
        .await?;

    let mut buffer_academic = String::new();
    println!("\nAcademic Agent Response:");
    while let Some(chunk) = response_academic.chunk().await? {
        match academic_agent.process_stream_response(&conversation_id_academic, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer_academic.push_str(&content);
            }
            Ok(None) => {
                academic_agent.add_message(&conversation_id_academic, "assistant", &buffer_academic).await;
                println!("\n--- End of Academic Agent Response ---");
                break;
            }
            Err(e) => {
                eprintln!("\nError processing stream for Academic Agent: {}", e);
                break;
            }
        }
    }

    // --- Tooling Agent Interaction ---
    info!("\n--- Tooling Agent ---");
    let conversation_id_tooling = tooling_agent.start_conversation(&model_name);
    info!("Tooling Agent Conversation ID: {}", conversation_id_tooling);

    print!("Enter your search query for the Tooling Agent (e.g., Latest developments in Rust programming): ");
    io::stdout().flush()?;
    let mut user_search_query = String::new();
    io::stdin().read_line(&mut user_search_query)?;
    let query_tooling = user_search_query.trim();

    if query_tooling.is_empty() {
        println!("No search query entered. Skipping web search.");
    } else {
        info!("Performing web search for: {}", query_tooling);
        let search_results = tooling_agent.search(query_tooling).await?;

        for result in &search_results {
            tooling_agent.add_message(&conversation_id_tooling, "search", format!("{} : {}", result.title, result.snippet).as_str()).await;
            println!("Title: {}", result.title);
            println!("URL: {}", result.url);
            println!();
        }

        tooling_agent.add_message(&conversation_id_tooling, "user", format!("Search for {} and summarize the first result.", query_tooling).as_str()).await;

        if let Some(first_result) = search_results.first() {
            info!("\nProcessing first search result: {}", first_result.url);
            match tooling_agent.fetch_page(&first_result.url).await {
                Ok(page) => {
                    tooling_agent.add_message(&conversation_id_tooling, "search", format!("Full page content from {}: {}...", page.url, page.content.chars().take(500).collect::<String>()).as_str()).await;

                    let role_tooling = Role::translator(Some(Audience::Family), Some(Preset::Simplify));
                    let prompt_summary = "Provide a simple summary of the fetched page content.";
                    let mut response_tooling = tooling_agent
                        .chat_with_history(&conversation_id_tooling, prompt_summary, Some(role_tooling))
                        .await?;

                    let mut buffer_tooling = String::new();
                    println!("\nTooling Agent Summary Response:");
                    while let Some(chunk) = response_tooling.chunk().await? {
                        match tooling_agent.process_stream_response(&conversation_id_tooling, &chunk).await {
                            Ok(Some(content)) => {
                                print!("{}", content);
                                io::stdout().flush()?;
                                buffer_tooling.push_str(&content);
                            }
                            Ok(None) => {
                                tooling_agent.add_message(&conversation_id_tooling, "assistant", &buffer_tooling).await;
                                println!("\n--- End of Tooling Agent Summary ---");
                                break;
                            }
                            Err(e) => {
                                eprintln!("\nError processing stream for Tooling Agent: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch page {}: {}", first_result.url, e);
                }
            }
        } else {
            println!("No search results to process.");
        }
    }

    println!("\nApplication finished.");
    Ok(())
}
