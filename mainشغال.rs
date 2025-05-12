///شغال
////content/kowalski/src/main.rsص


/// Main: The entry point of our AI-powered circus.
/// "Main functions are like orchestras - they make everything work together, but nobody notices until something goes wrong."
///
/// This is where the magic happens, or at least where we pretend it does.
/// Think of it as the conductor of our AI symphony, but with more error handling.
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
use std::io::{self, Write};
use utils::PdfReader; // لا يزال موجودًا لكن لن نستخدمه في هذا المثال مع AcademicAgent
use env_logger;
use log::info;

/// Reads input from a file, because apparently typing is too mainstream.
/// "File reading is like opening presents - you never know what you're gonna get."
///
/// # Arguments
/// * `file_path` - The path to the file (which is probably too long and boring)
///
/// # Returns
/// * `Result<String, Box<dyn std::error::Error>>` - Either the file contents or an error that will make you question your career choices
#[allow(dead_code)]
fn read_input_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if file_path.to_lowercase().ends_with(".pdf") {
        Ok(PdfReader::read_pdf_file(file_path)?)
    } else {
        Ok(fs::read_to_string(file_path)?)
    }
}

/// The main function that makes everything work (or at least tries to).
/// "Main functions are like first dates - they're exciting but usually end in disappointment."
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = config::Config::load()?;
    info!("Loaded configuration with search provider: {}", config.search.provider);

    // Initialize model manager
    let model_manager = ModelManager::new(config.ollama.base_url.clone())?;
    // let model_name = "michaelneale/deepseek-r1-goose";
    let model_name = "llama2:latest"; // quickest model

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
        println!("Pulling model {}...", model_name);
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
    }


    //  TODO: this is just temporary testing code
    //  TODO: remove this once we have a proper CLI interface
    //  TODO: and create examples of how to use the agents instead!
    // Initialize agents
    let mut academic_agent = AcademicAgent::new(config.clone())?;
    let mut tooling_agent = ToolingAgent::new(config)?; // config تم استهلاكه هنا، لذا نحتاج إلى config.clone() إذا استخدمنا academic_agent لاحقًا

    // Example: Process a research paper (تم التعديل لإرسال سؤال نصي)
    println!("\nSending simple question to Academic Agent...");
    let conversation_id_academic = academic_agent.start_conversation(&model_name); // تم تغيير اسم المتغير لتجنب التضارب
    println!("Academic Agent Conversation ID: {}", conversation_id_academic);

    let role_academic = Role::translator(Some(Audience::Scientist), Some(Preset::Questions)); // تم تغيير اسم المتغير
    
    // --- بداية التعديل ---
    let mut response_academic = academic_agent // تم تغيير اسم المتغير
        .chat_with_history(
            &conversation_id_academic,
            "What is game theory in one sentence?", // <--- سؤال بسيط كنص
            Some(role_academic),
        )
        .await?;
    // --- نهاية التعديل ---

    let mut buffer_academic = String::new(); // تم تغيير اسم المتغير
    while let Some(chunk) = response_academic.chunk().await? { // تم تغيير اسم المتغير
        match academic_agent.process_stream_response(&conversation_id_academic, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer_academic.push_str(&content);
            }
            Ok(None) => {
                academic_agent.add_message(&conversation_id_academic, "assistant", &buffer_academic).await;
                println!("\n");
                break;
            }
            Err(e) => {
                eprintln!("\nError processing stream: {}", e);
                break;
            }
        }
    }

    // Example: Web search and processing
    info!("Performing web search...");
    let conversation_id_tooling = tooling_agent.start_conversation(&model_name); // تم تغيير اسم المتغير
    info!("Tooling Agent Conversation ID: {}", conversation_id_tooling);

    let query = "Latest developments in Rust programming";
    let search_results = tooling_agent.search(query).await?;
  
    for result in &search_results {
        tooling_agent.add_message(&conversation_id_tooling, "search", format!("{} : {}", result.title, result.snippet).as_str()).await;
        println!("Title: {}", result.title);
        println!("URL: {}", result.url);
        println!("Snippet: {}", result.snippet);
        println!();
    }

    tooling_agent.add_message(&conversation_id_tooling, "user", format!("Search for {} and summary", query).as_str()).await;


    // Process the first search result
    if let Some(first_result) = search_results.first() {
        info!("\nProcessing first search result...");
        let page = tooling_agent.fetch_page(&first_result.url).await?;

        tooling_agent.add_message(&conversation_id_tooling, "search", format!(" Full page:{} : {}", page.title, page.content).as_str()).await;
        
        let role_tooling = Role::translator(Some(Audience::Family), Some(Preset::Simplify)); // تم تغيير اسم المتغير
        let mut response_tooling = tooling_agent // تم تغيير اسم المتغير
            .chat_with_history(&conversation_id_tooling, "Provide simple summary", Some(role_tooling))
            .await?;

        let mut buffer_tooling = String::new(); // تم تغيير اسم المتغير
        while let Some(chunk) = response_tooling.chunk().await? { // تم تغيير اسم المتغير
            match tooling_agent.process_stream_response(&conversation_id_tooling, &chunk).await {
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                    buffer_tooling.push_str(&content);
                }
                Ok(None) => {
                    tooling_agent.add_message(&conversation_id_tooling, "assistant", &buffer_tooling).await;
                    println!("\n");
                    break;
                }
                Err(e) => {
                    eprintln!("\nError processing stream: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
