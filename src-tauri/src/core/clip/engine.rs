use crate::core::clip::math::{cosine_similarity, l2_normalize, softmax};
use crate::core::clip::prompts::{all_category_prompts, value_drop_prompts, value_keep_prompts};
use crate::core::model::{CategoryKey, Scores, CATEGORY_KEYS};
use anyhow::{anyhow, Result};
use ort::execution_providers::{
    CPUExecutionProvider, CUDAExecutionProvider, CoreMLExecutionProvider,
    DirectMLExecutionProvider, ExecutionProvider, ExecutionProviderDispatch,
    OpenVINOExecutionProvider, ROCmExecutionProvider,
};
use ort::execution_providers::coreml::CoreMLModelFormat;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::run_options::{OutputSelector, RunOptions};
use ort::session::Session;
use ort::value::Tensor;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Manager};
use tokenizers::Tokenizer;

#[derive(Debug, Clone)]
pub struct ClipEngineOptions {
    pub model_dir: Option<String>,
    pub model_file: String,
    pub session_pool_size: usize,
    pub intra_threads: usize,
    pub enable_value: bool,
    pub allow_ep_fallback: bool,
    pub ep_auto: bool,
    pub ep_coreml: bool,
    pub ep_cuda: bool,
    pub ep_rocm: bool,
    pub ep_directml: bool,
    pub ep_openvino: bool,
}

impl Default for ClipEngineOptions {
    fn default() -> Self {
        Self {
            model_dir: None,
            model_file: "onnx/model_q4f16.onnx".to_string(),
            session_pool_size: 1,
            intra_threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .max(1)
                .min(4),
            enable_value: false,
            allow_ep_fallback: true,
            ep_auto: true,
            ep_coreml: cfg!(target_vendor = "apple"),
            ep_cuda: false,
            ep_rocm: false,
            ep_directml: false,
            ep_openvino: false,
        }
    }
}

pub struct ClipEngine {
    model_path: PathBuf,
    tokenizer_path: PathBuf,
    sessions: Vec<Mutex<Session>>,
    rr: AtomicUsize,
    input_ids_name: String,
    attention_mask_name: String,
    pixel_values_name: String,
    output_image_embeds: String,
    output_text_embeds: String,
    dummy_input_ids: Vec<i64>,
    dummy_attention_mask: Vec<i64>,
    category_text_embeds: HashMap<CategoryKey, Vec<f32>>,
    value_keep_embed: Vec<f32>,
    value_drop_embed: Vec<f32>,
    model_load_ms: u128,
    text_cache_ms: u128,
    eps_log: String,
}

impl ClipEngine {
    pub fn resolve_model_dir(app: &AppHandle, override_dir: Option<&str>) -> Result<PathBuf> {
        if let Some(raw) = override_dir {
            let p = PathBuf::from(raw);
            if p.join("tokenizer.json").exists() {
                return Ok(p);
            }
        }

        // Production: bundled resources
        if let Ok(resource_dir) = app.path().resource_dir() {
            let candidates = [
                resource_dir.join("models/clip-vit-b32-onnx"),
                resource_dir.join("clip-vit-b32-onnx"),
            ];
            for candidate in candidates {
                let tok = candidate.join("tokenizer.json");
                if tok.exists() {
                    return Ok(candidate);
                }
            }
        }

        // Dev: run under `src-tauri/`, so try relative paths
        let cwd = std::env::current_dir()?;
        let candidates = [
            cwd.join("models/clip-vit-b32-onnx"),
            cwd.join("../models/clip-vit-b32-onnx"),
            cwd.join("../../models/clip-vit-b32-onnx"),
        ];
        for c in candidates {
            if c.join("tokenizer.json").exists() {
                return Ok(c);
            }
        }

        Err(anyhow!(
            "CLIP model dir not found (expected `models/clip-vit-b32-onnx/` as Tauri resource or project path)"
        ))
    }

    pub fn new(app: &AppHandle, opts: ClipEngineOptions) -> Result<Self> {
        let dir = Self::resolve_model_dir(app, opts.model_dir.as_deref())?;
        let model_path = dir.join(Path::new(&opts.model_file));
        if !model_path.exists() {
            return Err(anyhow!(
                "CLIP ONNX model file not found: {}",
                model_path.display()
            ));
        }
        let tokenizer_path = dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(anyhow!(
                "CLIP tokenizer.json not found: {}",
                tokenizer_path.display()
            ));
        }

        let mut opts_try = opts.clone();
        let intra_threads = opts_try.intra_threads.max(1);

        let started = std::time::Instant::now();
        let (mut first_session, eps_log) = loop {
            let builder = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(intra_threads)?;

            let (eps, eps_log) = build_execution_providers(&opts_try);
            let builder = match builder.with_execution_providers(eps) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!(
                        "clip: failed to apply execution providers (fallback to CPU). err={}",
                        e
                    );
                    Session::builder()?
                        .with_optimization_level(GraphOptimizationLevel::Level3)?
                        .with_intra_threads(intra_threads)?
                }
            };

            let first_session = builder.commit_from_file(&model_path);
            match first_session {
                Ok(s) => break (s, eps_log),
                Err(e) => {
                    if opts_try.allow_ep_fallback && opts_try.ep_auto && opts_try.ep_coreml {
                        eprintln!(
                            "clip: session build failed with CoreML enabled, retrying without CoreML. err={}",
                            e
                        );
                        opts_try.ep_coreml = false;
                        continue;
                    }
                    return Err(e.into());
                }
            }
        };

        let model_load_ms = started.elapsed().as_millis();

        let (input_ids_name, attention_mask_name, pixel_values_name) =
            resolve_input_names(&first_session)?;

        let output_image_embeds = pick_output_name(
            first_session
                .outputs
                .iter()
                .map(|o| o.name.as_str())
                .collect(),
            &[
                "image_embeds",
                "image_embeddings",
                "image_features",
                "vision_embeds",
            ],
        )?;
        let output_text_embeds = pick_output_name(
            first_session
                .outputs
                .iter()
                .map(|o| o.name.as_str())
                .collect(),
            &["text_embeds", "text_embeddings", "text_features"],
        )?;

        let tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow!(e.to_string()))?;
        let pad_id = tokenizer
            .token_to_id("<|endoftext|>")
            .ok_or_else(|| anyhow!("tokenizer missing <|endoftext|>"))? as i64;

        // Prepare dummy text input (will be used when we only need image embeddings).
        let dummy = encode_fixed_77(&tokenizer, "", pad_id)?;

        // Cache text embeddings at init (will also validate EP compatibility).
        let started_cache = std::time::Instant::now();
        let (category_text_embeds, value_keep_embed, value_drop_embed) = loop {
            let category = cache_category_text_embeds(
                &mut first_session,
                &tokenizer,
                pad_id,
                &dummy.0,
                &dummy.1,
                &input_ids_name,
                &attention_mask_name,
                &pixel_values_name,
                &output_text_embeds,
            );
            let keep = cache_text_embed_for_prompts(
                &mut first_session,
                &tokenizer,
                pad_id,
                &input_ids_name,
                &attention_mask_name,
                &pixel_values_name,
                &output_text_embeds,
                value_keep_prompts(),
            );
            let drop = cache_text_embed_for_prompts(
                &mut first_session,
                &tokenizer,
                pad_id,
                &input_ids_name,
                &attention_mask_name,
                &pixel_values_name,
                &output_text_embeds,
                value_drop_prompts(),
            );

            match (category, keep, drop) {
                (Ok(c), Ok(k), Ok(d)) => {
                    // Smoke-test vision path as well. Some EPs can compile/load but fail at runtime.
                    if let Err(e) = smoke_test_vision(
                        &mut first_session,
                        &input_ids_name,
                        &attention_mask_name,
                        &pixel_values_name,
                        &output_image_embeds,
                        &dummy.0,
                        &dummy.1,
                    ) {
                        if opts_try.allow_ep_fallback && opts_try.ep_auto && opts_try.ep_coreml {
                            eprintln!(
                                "clip: CoreML failed during vision smoke test, retrying without CoreML. err={}",
                                e
                            );
                            opts_try.ep_coreml = false;
                            let builder = Session::builder()?
                                .with_optimization_level(GraphOptimizationLevel::Level3)?
                                .with_intra_threads(intra_threads)?;
                            let (eps, _eps_log2) = build_execution_providers(&opts_try);
                            let builder = builder.with_execution_providers(eps)?;
                            first_session = builder.commit_from_file(&model_path)?;
                            continue;
                        }
                        return Err(e);
                    }
                    break (c, k, d);
                }
                (cat_err, keep_err, drop_err) => {
                    let err = cat_err.err().or_else(|| keep_err.err()).or_else(|| drop_err.err());
                    let err = err.unwrap_or_else(|| anyhow!("unknown cache error"));
                    if opts_try.allow_ep_fallback && opts_try.ep_auto && opts_try.ep_coreml {
                        eprintln!(
                            "clip: CoreML failed during warmup, retrying without CoreML. err={}",
                            err
                        );
                        opts_try.ep_coreml = false;
                        // rebuild session without CoreML
                        let builder = Session::builder()?
                            .with_optimization_level(GraphOptimizationLevel::Level3)?
                            .with_intra_threads(intra_threads)?;
                        let (eps, _eps_log2) = build_execution_providers(&opts_try);
                        let builder = builder.with_execution_providers(eps)?;
                        first_session = builder.commit_from_file(&model_path)?;
                        continue;
                    }
                    return Err(err);
                }
            }
        };

        let text_cache_ms = started_cache.elapsed().as_millis();

        let session_pool_size = opts_try.session_pool_size.max(1);
        let mut sessions: Vec<Mutex<Session>> = Vec::with_capacity(session_pool_size);
        sessions.push(Mutex::new(first_session));
        for _ in 1..session_pool_size {
            let started = std::time::Instant::now();
            let builder = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(intra_threads)?;

            let (eps, _eps_log2) = build_execution_providers(&opts_try);
            let builder = match builder.with_execution_providers(eps) {
                Ok(b) => b,
                Err(_) => Session::builder()?
                    .with_optimization_level(GraphOptimizationLevel::Level3)?
                    .with_intra_threads(intra_threads)?,
            };
            let session = builder.commit_from_file(&model_path)?;
            let extra_ms = started.elapsed().as_millis();
            eprintln!(
                "clip: session pooled ({} of {}) loaded in {}ms",
                sessions.len() + 1,
                session_pool_size,
                extra_ms
            );
            sessions.push(Mutex::new(session));
        }

        eprintln!(
            "clip: loaded model in {}ms, cached text embeds in {}ms (model={}) eps={} pool={} intra_threads={}",
            model_load_ms,
            text_cache_ms,
            model_path.display(),
            eps_log,
            session_pool_size,
            intra_threads
        );

        Ok(Self {
            model_path,
            tokenizer_path,
            sessions,
            rr: AtomicUsize::new(0),
            input_ids_name,
            attention_mask_name,
            pixel_values_name,
            output_image_embeds,
            output_text_embeds,
            dummy_input_ids: dummy.0,
            dummy_attention_mask: dummy.1,
            category_text_embeds,
            value_keep_embed,
            value_drop_embed,
            model_load_ms,
            text_cache_ms,
            eps_log,
        })
    }

    pub fn classify(
        &self,
        image_nchw: &[f32],
    ) -> Result<(Scores, CategoryKey, Option<(bool, f32)>, String, u128)> {
        let started = std::time::Instant::now();
        let pixel = ndarray::Array4::<f32>::from_shape_vec((1, 3, 224, 224), image_nchw.to_vec())?;
        let pixel_tensor = Tensor::from_array(pixel)?;

        let ids = ndarray::Array2::<i64>::from_shape_vec((1, 77), self.dummy_input_ids.clone())?;
        let mask =
            ndarray::Array2::<i64>::from_shape_vec((1, 77), self.dummy_attention_mask.clone())?;
        let ids_tensor = Tensor::from_array(ids)?;
        let mask_tensor = Tensor::from_array(mask)?;

        let run_image_only = RunOptions::new()?
            .with_outputs(OutputSelector::no_default().with(self.output_image_embeds.as_str()));

        let idx = self.rr.fetch_add(1, Ordering::Relaxed) % self.sessions.len().max(1);
        let mut session = self
            .sessions
            .get(idx)
            .ok_or_else(|| anyhow!("clip session pool is empty"))?
            .lock();
        let outputs = session.run_with_options(
            ort::inputs![
                self.input_ids_name.as_str() => &ids_tensor,
                self.attention_mask_name.as_str() => &mask_tensor,
                self.pixel_values_name.as_str() => &pixel_tensor,
            ],
            &run_image_only,
        )?;

        let out = outputs
            .iter()
            .next()
            .map(|(_, v)| v)
            .ok_or_else(|| anyhow!("missing image embeddings output"))?;
        let (_shape, data) = out.try_extract_tensor::<f32>()?;
        if data.is_empty() {
            return Err(anyhow!("empty image embeddings"));
        }
        let mut image_embed = data.to_vec();
        l2_normalize(&mut image_embed);

        let value_logits = vec![
            cosine_similarity(&image_embed, &self.value_keep_embed),
            cosine_similarity(&image_embed, &self.value_drop_embed),
        ];
        let value_probs = softmax(&value_logits);
        let keep_prob = value_probs.get(0).copied().unwrap_or(0.0);
        let is_valuable = keep_prob >= 0.5;

        let mut logits = Vec::<f32>::with_capacity(CATEGORY_KEYS.len());
        for k in CATEGORY_KEYS {
            let t = self
                .category_text_embeds
                .get(k)
                .ok_or_else(|| anyhow!("missing text embedding for {}", k.as_str()))?;
            logits.push(cosine_similarity(&image_embed, t));
        }
        let probs = softmax(&logits);
        if probs.len() != CATEGORY_KEYS.len() {
            return Err(anyhow!("softmax length mismatch"));
        }
        let scores = Scores {
            screenshot_document: probs[0],
            people: probs[1],
            food_cafe: probs[2],
            nature_landscape: probs[3],
            city_street_travel: probs[4],
            pets_animals: probs[5],
            products_objects: probs[6],
            other: probs[7],
        };
        let (category, _top) = scores.top();

        let inference_ms = started.elapsed().as_millis();
        let log = format!(
            "engine: clip\nmodel_path: {model}\ntokenizer_path: {tok}\nmodel_load_ms: {load}\ntext_cache_ms: {cache}\nexecution_providers: {eps}\noutput_image_embeds: {oimg}\noutput_text_embeds: {otxt}\nvision_infer_ms: {infer}\nvalue_keep_prob: {keep_prob:.4}\n",
            model = self.model_path.display(),
            tok = self.tokenizer_path.display(),
            load = self.model_load_ms,
            cache = self.text_cache_ms,
            eps = self.eps_log,
            oimg = self.output_image_embeds,
            otxt = self.output_text_embeds,
            infer = inference_ms,
            keep_prob = keep_prob,
        );
        Ok((scores, category, Some((is_valuable, keep_prob)), log, inference_ms))
    }
}

fn pick_output_name<'a>(outputs: Vec<&'a str>, priorities: &[&str]) -> Result<String> {
    for p in priorities {
        if let Some(name) = outputs
            .iter()
            .find(|n| n.eq_ignore_ascii_case(p) || n.to_lowercase().contains(&p.to_lowercase()))
        {
            return Ok((*name).to_string());
        }
    }
    Err(anyhow!(
        "required output not found. available outputs: {}",
        outputs.join(", ")
    ))
}

fn encode_fixed_77(tokenizer: &Tokenizer, text: &str, pad_id: i64) -> Result<(Vec<i64>, Vec<i64>)> {
    let encoding = tokenizer
        .encode(text, true)
        .map_err(|e| anyhow!(e.to_string()))?;
    let mut ids: Vec<i64> = encoding.get_ids().iter().map(|v| *v as i64).collect();
    let mut mask: Vec<i64> = encoding
        .get_attention_mask()
        .iter()
        .map(|v| *v as i64)
        .collect();

    const MAX_LEN: usize = 77;
    if ids.len() > MAX_LEN {
        ids.truncate(MAX_LEN);
        mask.truncate(MAX_LEN);
    }
    while ids.len() < MAX_LEN {
        ids.push(pad_id);
        mask.push(0);
    }
    while mask.len() < MAX_LEN {
        mask.push(0);
    }
    Ok((ids, mask))
}

fn cache_category_text_embeds(
    session: &mut Session,
    tokenizer: &Tokenizer,
    pad_id: i64,
    dummy_ids: &[i64],
    dummy_mask: &[i64],
    input_ids_name: &str,
    attention_mask_name: &str,
    pixel_values_name: &str,
    output_text_embeds: &str,
) -> Result<HashMap<CategoryKey, Vec<f32>>> {
    // Flatten prompts
    let prompt_sets = all_category_prompts();
    let mut flat_prompts: Vec<(CategoryKey, String)> = Vec::new();
    for (k, arr) in prompt_sets {
        for s in arr {
            flat_prompts.push((k, s.to_string()));
        }
    }
    if flat_prompts.is_empty() {
        return Err(anyhow!("no prompts"));
    }

    let mut ids_all: Vec<i64> = Vec::with_capacity(flat_prompts.len() * 77);
    let mut mask_all: Vec<i64> = Vec::with_capacity(flat_prompts.len() * 77);
    for (_, p) in flat_prompts.iter() {
        let (ids, mask) = encode_fixed_77(tokenizer, p, pad_id)?;
        ids_all.extend_from_slice(&ids);
        mask_all.extend_from_slice(&mask);
    }
    let n = flat_prompts.len();
    let ids = ndarray::Array2::<i64>::from_shape_vec((n, 77), ids_all)?;
    let mask = ndarray::Array2::<i64>::from_shape_vec((n, 77), mask_all)?;
    let ids_tensor = Tensor::from_array(ids)?;
    let mask_tensor = Tensor::from_array(mask)?;

    // Dummy pixel values (text-only run, only outputs text embeddings)
    // Some exported CLIP ONNX graphs require matching batch sizes for all inputs,
    // so we size pixel_values to the same batch as text.
    let dummy_pixel = ndarray::Array4::<f32>::zeros((n, 3, 224, 224));
    let pixel_tensor = Tensor::from_array(dummy_pixel)?;

    let run_text_only =
        RunOptions::new()?.with_outputs(OutputSelector::no_default().with(output_text_embeds));

    let outputs = session.run_with_options(
        ort::inputs![
            input_ids_name => &ids_tensor,
            attention_mask_name => &mask_tensor,
            pixel_values_name => &pixel_tensor,
        ],
        &run_text_only,
    )?;
    let out = outputs
        .iter()
        .next()
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow!("missing text embeddings output"))?;
    let (_shape, data) = out.try_extract_tensor::<f32>()?;
    if data.is_empty() {
        return Err(anyhow!("empty text embeddings"));
    }

    // shape is expected [n, d]
    let d = data.len() / n;
    if d == 0 {
        return Err(anyhow!("invalid text embeddings shape"));
    }

    // Aggregate embeddings per category: average across prompts, then L2 normalize.
    let mut sums: HashMap<CategoryKey, Vec<f32>> = HashMap::new();
    let mut counts: HashMap<CategoryKey, usize> = HashMap::new();
    for (i, (k, _)) in flat_prompts.into_iter().enumerate() {
        let start = i * d;
        let end = start + d;
        let vec = &data[start..end];
        let entry = sums.entry(k).or_insert_with(|| vec![0.0f32; d]);
        for j in 0..d {
            entry[j] += vec[j];
        }
        *counts.entry(k).or_insert(0) += 1;
    }

    let mut out_map: HashMap<CategoryKey, Vec<f32>> = HashMap::new();
    for k in CATEGORY_KEYS {
        let mut v = sums
            .remove(k)
            .ok_or_else(|| anyhow!("missing text sum for {}", k.as_str()))?;
        let c = *counts.get(k).unwrap_or(&1) as f32;
        for x in v.iter_mut() {
            *x /= c;
        }
        l2_normalize(&mut v);
        out_map.insert(*k, v);
    }

    // Sanity: ensure dummy ids/mask length is correct (avoid unused vars).
    if dummy_ids.len() != 77 || dummy_mask.len() != 77 {
        return Err(anyhow!("dummy text input must be length 77"));
    }

    Ok(out_map)
}

fn cache_text_embed_for_prompts(
    session: &mut Session,
    tokenizer: &Tokenizer,
    pad_id: i64,
    input_ids_name: &str,
    attention_mask_name: &str,
    pixel_values_name: &str,
    output_text_embeds: &str,
    prompts: &[&str],
) -> Result<Vec<f32>> {
    if prompts.is_empty() {
        return Err(anyhow!("no prompts for embed cache"));
    }

    let n = prompts.len();
    let mut ids_all: Vec<i64> = Vec::with_capacity(n * 77);
    let mut mask_all: Vec<i64> = Vec::with_capacity(n * 77);
    for p in prompts.iter() {
        let (ids, mask) = encode_fixed_77(tokenizer, p, pad_id)?;
        ids_all.extend_from_slice(&ids);
        mask_all.extend_from_slice(&mask);
    }
    let ids = ndarray::Array2::<i64>::from_shape_vec((n, 77), ids_all)?;
    let mask = ndarray::Array2::<i64>::from_shape_vec((n, 77), mask_all)?;
    let ids_tensor = Tensor::from_array(ids)?;
    let mask_tensor = Tensor::from_array(mask)?;

    // Some exported CLIP ONNX graphs require matching batch sizes for all inputs,
    // so we size pixel_values to the same batch as text.
    let dummy_pixel = ndarray::Array4::<f32>::zeros((n, 3, 224, 224));
    let pixel_tensor = Tensor::from_array(dummy_pixel)?;

    let run_text_only =
        RunOptions::new()?.with_outputs(OutputSelector::no_default().with(output_text_embeds));
    let outputs = session.run_with_options(
        ort::inputs![
            input_ids_name => &ids_tensor,
            attention_mask_name => &mask_tensor,
            pixel_values_name => &pixel_tensor,
        ],
        &run_text_only,
    )?;
    let out = outputs
        .iter()
        .next()
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow!("missing text embeddings output"))?;
    let (_shape, data) = out.try_extract_tensor::<f32>()?;
    if data.is_empty() {
        return Err(anyhow!("empty text embeddings"));
    }

    let d = data.len() / n;
    if d == 0 {
        return Err(anyhow!("invalid text embeddings shape"));
    }

    let mut avg = vec![0.0f32; d];
    for i in 0..n {
        let start = i * d;
        for j in 0..d {
            avg[j] += data[start + j];
        }
    }
    let denom = (n as f32).max(1.0);
    for x in avg.iter_mut() {
        *x /= denom;
    }
    l2_normalize(&mut avg);
    Ok(avg)
}

fn smoke_test_vision(
    session: &mut Session,
    input_ids_name: &str,
    attention_mask_name: &str,
    pixel_values_name: &str,
    output_image_embeds: &str,
    dummy_input_ids: &[i64],
    dummy_attention_mask: &[i64],
) -> Result<()> {
    let pixel = ndarray::Array4::<f32>::zeros((1, 3, 224, 224));
    let pixel_tensor = Tensor::from_array(pixel)?;

    let ids = ndarray::Array2::<i64>::from_shape_vec((1, 77), dummy_input_ids.to_vec())?;
    let mask = ndarray::Array2::<i64>::from_shape_vec((1, 77), dummy_attention_mask.to_vec())?;
    let ids_tensor = Tensor::from_array(ids)?;
    let mask_tensor = Tensor::from_array(mask)?;

    let run_image_only =
        RunOptions::new()?.with_outputs(OutputSelector::no_default().with(output_image_embeds));

    let outputs = session.run_with_options(
        ort::inputs![
            input_ids_name => &ids_tensor,
            attention_mask_name => &mask_tensor,
            pixel_values_name => &pixel_tensor,
        ],
        &run_image_only,
    )?;

    let out = outputs
        .iter()
        .next()
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow!("missing image embeddings output"))?;
    let (_shape, data) = out.try_extract_tensor::<f32>()?;
    if data.is_empty() {
        return Err(anyhow!("empty image embeddings (smoke test)"));
    }
    Ok(())
}

fn resolve_input_names(session: &Session) -> Result<(String, String, String)> {
    let mut input_ids = None::<String>;
    let mut mask = None::<String>;
    let mut pixel = None::<String>;
    for i in &session.inputs {
        let name = i.name.as_str();
        let lower = name.to_lowercase();
        if input_ids.is_none() && (lower.contains("input_ids") || lower == "input") {
            input_ids = Some(name.to_string());
        } else if mask.is_none() && lower.contains("attention_mask") {
            mask = Some(name.to_string());
        } else if pixel.is_none() && (lower.contains("pixel_values") || lower.contains("pixel")) {
            pixel = Some(name.to_string());
        }
    }
    Ok((
        input_ids.ok_or_else(|| anyhow!("model input_ids not found"))?,
        mask.ok_or_else(|| anyhow!("model attention_mask not found"))?,
        pixel.ok_or_else(|| anyhow!("model pixel_values not found"))?,
    ))
}

fn provider_cap(ep: &impl ExecutionProvider) -> (bool, bool) {
    let supported = ep.supported_by_platform();
    let available = if supported {
        ep.is_available().unwrap_or(false)
    } else {
        false
    };
    (supported, available)
}

fn build_execution_providers(opts: &ClipEngineOptions) -> (Vec<ExecutionProviderDispatch>, String) {
    let mut eps: Vec<ExecutionProviderDispatch> = Vec::new();
    let mut enabled: Vec<&'static str> = Vec::new();

    if opts.ep_auto {
        if opts.ep_coreml {
            // MLProgram supports more operators than NeuralNetwork and generally improves
            // compatibility for transformer-style graphs on modern macOS.
            let ep = CoreMLExecutionProvider::default()
                .with_model_format(CoreMLModelFormat::MLProgram)
                .with_static_input_shapes(true);
            let (supported, available) = provider_cap(&ep);
            if supported && available {
                eps.push(ep.build());
                enabled.push("coreml");
            }
        }
        if opts.ep_cuda {
            let ep = CUDAExecutionProvider::default();
            let (supported, available) = provider_cap(&ep);
            if supported && available {
                eps.push(ep.build());
                enabled.push("cuda");
            }
        }
        if opts.ep_rocm {
            let ep = ROCmExecutionProvider::default();
            let (supported, available) = provider_cap(&ep);
            if supported && available {
                eps.push(ep.build());
                enabled.push("rocm");
            }
        }
        if opts.ep_directml {
            let ep = DirectMLExecutionProvider::default();
            let (supported, available) = provider_cap(&ep);
            if supported && available {
                eps.push(ep.build());
                enabled.push("directml");
            }
        }
        if opts.ep_openvino {
            let ep = OpenVINOExecutionProvider::default();
            let (supported, available) = provider_cap(&ep);
            if supported && available {
                eps.push(ep.build());
                enabled.push("openvino");
            }
        }
    }

    // Always include CPU as last fallback.
    eps.push(CPUExecutionProvider::default().build());
    if enabled.is_empty() {
        (eps, "cpu".to_string())
    } else {
        (eps, format!("{}+cpu", enabled.join("+")))
    }
}
