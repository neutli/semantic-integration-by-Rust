use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Write, BufReader, BufRead};
use serde::{Serialize, Deserialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/*
 * ============================================================================
 * Everling Semantic Integration (ESI) - v1.1.2 (デバッグ・同期強化版)
 * ============================================================================
 * [理論概要]
 * 形態素解析済みのコーパスデータから語彙を抽出し、高次元空間での積分を通じて
 * 意味を合成します。
 * ============================================================================
 */

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
enum DiscourseMode {
    Narrative, // 叙述的（高慣性）
    Dialectic, // 弁証法的（高ノイズ）
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum Language {
    English,
    Japanese,
    Chinese,
}

// --- 構造体定義 ---

#[derive(Debug, Clone, Serialize)]
struct ExperimentConfig {
    virtual_dimensions: usize,
    active_dimensions: usize,
    integration_steps: usize,
    mode: DiscourseMode,
    noise_scale: f64,
    alpha: f64, 
    seed_text: String,
}

#[derive(Debug, Serialize)]
struct SimulationMetric {
    step: usize,
    variance: f64,
    structure_score: f64,
}

#[derive(Debug, Serialize)]
struct ResearchReport {
    config: ExperimentConfig,
    metrics: Vec<SimulationMetric>,
    generated_sentence: String,
    variance_change: f64, 
    intensity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VocabularyData {
    nouns: Vec<String>,
    particles: Vec<String>,
    verbs: Vec<String>,
    adverbs: Vec<String>,
}

impl VocabularyData {
    #[allow(dead_code)]
    fn empty() -> Self {
        VocabularyData { nouns: vec![], particles: vec![], verbs: vec![], adverbs: vec![] }
    }

    /// 別の語彙データを自身にマージし、重複を排除する
    fn merge(&mut self, other: VocabularyData) {
        let mut n: HashSet<_> = self.nouns.drain(..).collect();
        let mut p: HashSet<_> = self.particles.drain(..).collect();
        let mut v: HashSet<_> = self.verbs.drain(..).collect();
        let mut a: HashSet<_> = self.adverbs.drain(..).collect();

        n.extend(other.nouns);
        p.extend(other.particles);
        v.extend(other.verbs);
        a.extend(other.adverbs);

        self.nouns = n.into_iter().collect();
        self.particles = p.into_iter().collect();
        self.verbs = v.into_iter().collect();
        self.adverbs = a.into_iter().collect();
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LanguageVocabularies {
    english: VocabularyData,
    japanese: VocabularyData,
    chinese: VocabularyData,
}

// ==================== 数学エンジン ====================

struct EverlingIntegrator {
    momentum: Vec<f64>,
    terrain: Vec<f64>,
}

impl EverlingIntegrator {
    fn new(dim: usize) -> Self {
        EverlingIntegrator {
            momentum: vec![0.0; dim],
            terrain: vec![0.0; dim],
        }
    }

    fn integrate(&mut self, alpha: f64, noise_scale: f64, rng: &mut impl Rng) {
        for (m, t) in self.momentum.iter_mut().zip(self.terrain.iter_mut()) {
            let gradient: f64 = rng.gen_range(-noise_scale..noise_scale);
            *m = (alpha * *m) + ((1.0 - alpha) * gradient);
            *t = (*t + *m).clamp(-1.0, 1.0);
        }
    }
}

// ==================== 形態素データ解析 ====================

struct MorphemeProcessor;

impl MorphemeProcessor {
    /// UniDic形式のデータから語彙を抽出する（カンマとタブの両方に対応）
    fn process_file(path: &str) -> io::Result<VocabularyData> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut nouns = HashSet::new();
        let mut particles = HashSet::new();
        let mut verbs = HashSet::new();
        let mut adverbs = HashSet::new();
        let mut line_count = 0;

        for line in reader.lines().skip(1) {
            let line = line?;
            line_count += 1;
            
            // カンマまたはタブで分割を試みる
            let parts: Vec<&str> = if line.contains(',') {
                line.split(',').collect()
            } else {
                line.split('\t').collect()
            };
            
            // UniDicカラム想定: 2:表層形, 6:品詞
            if parts.len() > 6 {
                let word = parts[2].trim().replace("\"", "");
                if word.is_empty() { continue; }
                let pos = parts[6];

                if pos.contains("名詞") || pos.contains("代名詞") || pos.contains("接尾辞") {
                    nouns.insert(word);
                } else if pos.contains("助詞") {
                    particles.insert(word);
                } else if pos.contains("動詞") {
                    verbs.insert(word);
                } else if pos.contains("副詞") {
                    adverbs.insert(word);
                }
            }
        }

        println!("[Debug] {} 行のデータを走査しました。", line_count);

        Ok(VocabularyData {
            nouns: nouns.into_iter().collect(),
            particles: particles.into_iter().collect(),
            verbs: verbs.into_iter().collect(),
            adverbs: adverbs.into_iter().collect(),
        })
    }
}

// ==================== 言語合成エンジン ====================

struct LinguisticAssembler {
    language: Language,
    vocab: VocabularyData,
}

impl LinguisticAssembler {
    fn new(language: Language) -> Result<Self, Box<dyn std::error::Error>> {
        let vocab = Self::load_and_sync_vocabulary(language)?;
        Ok(LinguisticAssembler { language, vocab })
    }

    fn load_and_sync_vocabulary(language: Language) -> Result<VocabularyData, Box<dyn std::error::Error>> {
        let vocab_path = Path::new("vocabulary.json");
        let raw_data_path = "morphemes.csv"; 

        // 1. 既存のJSONデータを読み込む
        let mut all_vocabs = if vocab_path.exists() {
            let file = File::open(vocab_path)?;
            serde_json::from_reader(BufReader::new(file))?
        } else {
            Self::get_default_all_vocabs()
        };

        // 2. CSVファイルがあれば解析
        if Path::new(raw_data_path).exists() {
            println!("[Sync] '{}' を検出しました。統合を開始します...", raw_data_path);
            let extracted = MorphemeProcessor::process_file(raw_data_path)?;
            
            let total_extracted = extracted.nouns.len() + extracted.particles.len() + extracted.verbs.len() + extracted.adverbs.len();
            
            if total_extracted > 0 {
                match language {
                    Language::Japanese => all_vocabs.japanese.merge(extracted),
                    Language::English => all_vocabs.english.merge(extracted),
                    Language::Chinese => all_vocabs.chinese.merge(extracted),
                }

                let json = serde_json::to_string_pretty(&all_vocabs)?;
                File::create(vocab_path)?.write_all(json.as_bytes())?;
                println!("[Sync] 新たに {} 種類の語彙を統合・永続化しました。", total_extracted);
            } else {
                println!("[Warning] ファイルは見つかりましたが、有効な語彙を抽出できませんでした。フォーマットを確認してください。");
            }
        } else {
            // ファイルパスのデバッグ表示
            let current_dir = std::env::current_dir()?;
            println!("[Debug] '{}' が見つかりません。現在のディレクトリ: {:?}", raw_data_path, current_dir);
        }

        let target_vocab = match language {
            Language::English => all_vocabs.english,
            Language::Japanese => all_vocabs.japanese,
            Language::Chinese => all_vocabs.chinese,
        };

        Ok(target_vocab)
    }

    fn get_default_all_vocabs() -> LanguageVocabularies {
        LanguageVocabularies {
            english: VocabularyData { nouns: vec!["Silence".into()], particles: vec!["is".into()], verbs: vec!["drifting".into()], adverbs: vec!["Gently".into()] },
            japanese: VocabularyData { 
                nouns: vec!["静寂".into(), "思考".into(), "技術".into(), "日常".into()], 
                particles: vec!["は".into(), "の".into(), "に".into()], 
                verbs: vec!["加速する".into(), "共鳴する".into()], 
                adverbs: vec!["徐々に".into(), "突如として".into()] 
            },
            chinese: VocabularyData { nouns: vec!["宁静".into()], particles: vec!["是".into()], verbs: vec!["漂流".into()], adverbs: vec!["温柔地".into()] },
        }
    }

    fn assemble(&self, state: &HashMap<usize, f64>) -> String {
        let mut sorted: Vec<(&usize, &f64)> = state.iter().collect();
        sorted.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());

        let mut output = String::new();
        let intensity = sorted[0].1.abs();
        
        if self.vocab.adverbs.is_empty() || self.vocab.nouns.is_empty() {
            return "語彙が不足しています。".to_string();
        }

        let adv_idx = (intensity * 100.0) as usize;
        output.push_str(&self.vocab.adverbs[adv_idx % self.vocab.adverbs.len()]);
        output.push_str(if matches!(self.language, Language::English) { ", " } else { "、" });

        for i in 0..3 {
            let (&dim, _) = sorted[i];
            output.push_str(&self.vocab.nouns[dim % self.vocab.nouns.len()]);
            output.push_str(&self.vocab.particles[(dim + i) % self.vocab.particles.len()]);
        }
        
        output.push_str(&self.vocab.verbs[*sorted[0].0 % self.vocab.verbs.len()]);
        output.push_str(if matches!(self.language, Language::English) { "." } else { "。" });
        output
    }
}

// ==================== 統計分析 ====================

fn calculate_variance(state: &HashMap<usize, f64>) -> f64 {
    let n = state.len() as f64;
    if n == 0.0 { return 0.0; }
    let mean = state.values().sum::<f64>() / n;
    state.values().map(|v| (v - mean).powi(2)).sum::<f64>() / n
}

// ==================== 実行コア ====================

fn run_experiment(config: ExperimentConfig, lang: Language) -> Result<ResearchReport, Box<dyn std::error::Error>> {
    let mut integrator = EverlingIntegrator::new(config.active_dimensions);
    let assembler = LinguisticAssembler::new(lang)?;
    let mut rng = rand::thread_rng();

    let mut hasher = DefaultHasher::new();
    config.seed_text.hash(&mut hasher);
    let seed_val = hasher.finish();

    let mut agent_state: HashMap<usize, f64> = HashMap::new();
    let dims: Vec<usize> = (0..config.active_dimensions)
        .map(|i| ((seed_val as usize).wrapping_add(i * 137)) % config.virtual_dimensions)
        .collect();

    for &d in &dims {
        agent_state.insert(d, rng.gen_range(-0.1..0.1)); 
    }

    let initial_variance = calculate_variance(&agent_state);
    let mut metrics = Vec::new();

    for step in 0..config.integration_steps {
        integrator.integrate(config.alpha, config.noise_scale, &mut rng);
        for (i, &d) in dims.iter().enumerate() {
            if let Some(val) = agent_state.get_mut(&d) {
                let target = integrator.terrain[i];
                *val = (*val * 0.9) + (target * 0.1);
            }
        }
        if step % 25 == 0 {
            let current_var = calculate_variance(&agent_state);
            metrics.push(SimulationMetric { step, variance: current_var, structure_score: current_var / initial_variance.max(1e-9) });
        }
    }

    Ok(ResearchReport {
        config: config.clone(),
        metrics,
        generated_sentence: assembler.assemble(&agent_state),
        variance_change: calculate_variance(&agent_state) / initial_variance.max(1e-9),
        intensity_score: agent_state.values().map(|v| v.abs()).sum::<f64>() / config.active_dimensions as f64,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Everling Semantic Research v1.1.2 ===");
    
    println!("\nシード値を入力してください:");
    let mut seed_text = String::new();
    io::stdin().read_line(&mut seed_text)?;
    let seed_text = seed_text.trim().to_string();

    fs::create_dir_all("results")?;

    let config = ExperimentConfig {
        virtual_dimensions: 18446744073709551615,
        active_dimensions: 128,
        integration_steps: 500,
        mode: DiscourseMode::Narrative,
        noise_scale: 0.05,
        alpha: 0.95,
        seed_text: seed_text.clone(),
    };

    let report = run_experiment(config, Language::Japanese)?;
    
    println!("\n[実験結果]");
    println!("  生成文: \"{}\"", report.generated_sentence);
    println!("  構造出現係数: {:.2}x", report.variance_change);

    let json = serde_json::to_string_pretty(&report)?;
    let mut file = File::create("results/report_morpheme_test.json")?;
    write!(file, "{}", json)?;

    println!("\n完了。抽出された語彙は 'vocabulary.json' に蓄積されています。");
    Ok(())
}