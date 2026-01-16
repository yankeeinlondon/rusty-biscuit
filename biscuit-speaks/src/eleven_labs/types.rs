use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Model {
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub can_do_text_to_speech: bool,
    pub can_do_voice_conversion: bool,
    pub token_cost_factor: f32,
    pub languages: Vec<Language>,
}

#[derive(Deserialize, Debug)]
pub struct Language {
    pub language_id: String,
    pub name: String,
}

use reqwest::header::CONTENT_TYPE;

// Include the structs from above here...


async fn eleven_labs_models() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Note: You can often call this without an API key, but we include it for consistency.
    let api_key = "YOUR_XI_API_KEY";

    let response = client
        .get("https://api.elevenlabs.io/v1/models")
        .header("xi-api-key", api_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        // Deserialize into a Vector of Models
        let models: Vec<Model> = response.json().await?;

        println!("Found {} models:\n", models.len());

        for model in models {
            println!("ID:          {}", model.model_id);
            println!("Name:        {}", model.name);
            println!("Description: {}", model.description);
            println!("Cost Factor: {}", model.token_cost_factor);
            println!("---");
        }
    } else {
        println!("Error: {:?}", response.status());
    }

    Ok(())
}
