use crate::kobold_api::{create_input, Client as KoboldClient, SseGenerationExt};
use crate::models::new_uuid_string;
use anyhow::Result;
use async_recursion::async_recursion;
use serde::de::DeserializeOwned;
use serde_json::error::Category;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// Characters which can break the JSON deserialization. Do not rely
/// on model to NOT print these (though it shouldn't). Make sure they
/// are removed from JSON responses.
const ILLEGAL_CHARACTERS: [char; 1] = ['\\'];

fn sanitize_json_response(mut json: String) -> String {
    for illegal_char in ILLEGAL_CHARACTERS {
        json = json.replace(illegal_char, "");
    }

    json
}

struct AiExecution<'a> {
    client: &'a KoboldClient,
    gen_key: &'a str,
    prompt: &'a AiPrompt,
    prompt_so_far: &'a mut String,
}

async fn converse<'a, T: DeserializeOwned>(details: &mut AiExecution<'a>) -> Result<T> {
    // Handle Mistral-instruct begin instruct mode.
    // https://huggingface.co/mistralai/Mistral-7B-Instruct-v0.2
    // Only the very first one begins with <s>, not subsequent.
    if details.prompt_so_far.is_empty() {
        details.prompt_so_far.push_str("<s>");
    }

    details.prompt_so_far.push_str(&details.prompt.prompt);

    let input = create_input(
        details.gen_key.to_string(),
        &details.prompt_so_far,
        details.prompt.grammar.clone(),
        details.prompt.max_tokens,
        false,
        details.prompt.creativity,
    );

    // TODO working on removing trait bounds issue so we can use ? operator.
    let mut str_resp = details
        .client
        .sse_generate(input)
        .await
        .map(sanitize_json_response)
        .unwrap();

    details.prompt_so_far.push_str(&str_resp);

    let resp: T = match serde_json::from_str(&str_resp) {
        Ok(obj) => obj,
        Err(e) => {
            // If the resp is not fully valid JSON, request more
            // from the LLM.
            match e.classify() {
                Category::Eof => {
                    continue_execution(
                        details,
                        &mut str_resp,
                    )
                    .await?
                }
                _ => return Err(e.into()),
            }
        }
    };

    // mistral 7b end of response token (for when BNF is used)
    if !details.prompt_so_far.trim().ends_with("</s>") {
        details.prompt_so_far.push_str("</s>");
    }

    Ok(resp)
}

#[async_recursion(?Send)]
async fn continue_execution<'a, T: DeserializeOwned>(
    details: &mut AiExecution<'a>,
    resp_so_far: &mut String,
) -> Result<T> {
    // Grammar state is retained here (as opposed to false
    // normally) to let the model continue to generate JSON.
    let input = create_input(
        details.gen_key.to_string(),
        details.prompt_so_far,
        details.prompt.grammar.clone(),
        details.prompt.max_tokens,
        true,
        details.prompt.creativity,
    );

    // TODO convert error to remove trait bound issue
    let resp = details.client.sse_generate(input).await.unwrap();

    details.prompt_so_far.push_str(&resp);
    resp_so_far.push_str(&resp);

    let resp: Value = match serde_json::from_str(&resp_so_far) {
        Ok(obj) => obj,
        Err(e) => match e.classify() {
            Category::Eof | Category::Syntax => {
                continue_execution(details, resp_so_far).await?
            }
            _ => {
                return Err(e.into());
            }
        },
    };

    let resp: T = serde_json::from_value(resp)?;

    Ok(resp)
}

#[derive(Debug, Clone, Copy)]
pub enum AiCreativity {
    Predictable,
    Normal,
    Creative,
}

pub struct AiPrompt {
    pub prompt: String,
    pub grammar: Option<String>,
    pub max_tokens: u64,
    pub creativity: AiCreativity,
}

impl AiPrompt {
    pub fn new(prompt: &str) -> AiPrompt {
        AiPrompt {
            prompt: prompt.to_string(),
            grammar: None,
            max_tokens: 150,
            creativity: AiCreativity::Normal,
        }
    }

    pub fn new_with_grammar(prompt: &str, grammar: &str) -> AiPrompt {
        AiPrompt {
            prompt: prompt.to_string(),
            grammar: Some(grammar.to_string()),
            max_tokens: 150,
            creativity: AiCreativity::Normal,
        }
    }

    pub fn new_with_grammar_and_size(prompt: &str, grammar: &str, tokens: u64) -> AiPrompt {
        AiPrompt {
            prompt: prompt.to_string(),
            grammar: Some(grammar.to_string()),
            max_tokens: tokens,
            creativity: AiCreativity::Normal,
        }
    }

    pub fn creative_with_grammar(prompt: &str, grammar: &str) -> AiPrompt {
        AiPrompt {
            prompt: prompt.to_string(),
            grammar: Some(grammar.to_string()),
            max_tokens: 150,
            creativity: AiCreativity::Creative,
        }
    }

    pub fn creative_with_grammar_and_size(prompt: &str, grammar: &str, tokens: u64) -> AiPrompt {
        AiPrompt {
            prompt: prompt.to_string(),
            grammar: Some(grammar.to_string()),
            max_tokens: tokens,
            creativity: AiCreativity::Creative,
        }
    }
}

pub struct AiConversation {
    gen_key: String,
    prompt_so_far: Rc<RefCell<String>>,
    client: Rc<KoboldClient>,
}

impl AiConversation {
    pub fn new(client: Rc<KoboldClient>) -> AiConversation {
        AiConversation {
            prompt_so_far: Rc::new(RefCell::new(String::new())),
            gen_key: new_uuid_string(),
            client,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.prompt_so_far.borrow().is_empty()
    }

    pub fn reset(&self) {
        let mut prompt_so_far = RefCell::borrow_mut(&self.prompt_so_far);
        prompt_so_far.clear();
    }

    pub async fn execute<T: DeserializeOwned>(&self, prompt: &AiPrompt) -> Result<T> {
        let mut prompt_so_far = RefCell::borrow_mut(&self.prompt_so_far);
        let prompt_so_far = &mut *prompt_so_far;

        let mut details = AiExecution {
            prompt_so_far,
            client: &self.client,
            gen_key: &self.gen_key,
            prompt: &prompt
        };

        converse(&mut details).await
    }
}
