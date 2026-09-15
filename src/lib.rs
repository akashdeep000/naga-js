#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::{Either, Uint8Array};
use napi_derive::napi;

// ---------------------------------------------------------------------------
// JS-facing types (these drive the generated `index.d.ts`)
// ---------------------------------------------------------------------------

/// Shader language accepted as *input*: `"wgsl"`, `"glsl"`, or `"spv"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
  Wgsl,
  Glsl,
  Spv,
}

/// Shader language produced as *output*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
  Wgsl,
  Glsl,
  Hlsl,
  Msl,
  Spv,
  Dot,
}

/// Pipeline stage of an entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
  Vertex,
  Fragment,
  Compute,
}

fn parse_input_format(value: &str) -> Result<InputFormat, Diagnostic> {
  match value {
    "wgsl" => Ok(InputFormat::Wgsl),
    "glsl" => Ok(InputFormat::Glsl),
    "spv" => Ok(InputFormat::Spv),
    other => Err(Diagnostic::usage(format!(
      "invalid `from` '{other}'; expected one of \"wgsl\", \"glsl\", \"spv\""
    ))),
  }
}

fn parse_output_format(value: &str) -> Result<OutputFormat, Diagnostic> {
  match value {
    "wgsl" => Ok(OutputFormat::Wgsl),
    "glsl" => Ok(OutputFormat::Glsl),
    "hlsl" => Ok(OutputFormat::Hlsl),
    "msl" => Ok(OutputFormat::Msl),
    "spv" => Ok(OutputFormat::Spv),
    "dot" => Ok(OutputFormat::Dot),
    other => Err(Diagnostic::usage(format!(
      "invalid `to` '{other}'; expected one of \"wgsl\", \"glsl\", \"hlsl\", \"msl\", \"spv\", \"dot\""
    ))),
  }
}

fn parse_shader_stage(value: &str) -> Result<ShaderStage, Diagnostic> {
  match value {
    "vertex" => Ok(ShaderStage::Vertex),
    "fragment" => Ok(ShaderStage::Fragment),
    "compute" => Ok(ShaderStage::Compute),
    other => Err(Diagnostic::usage(format!(
      "invalid `stage` '{other}'; expected one of \"vertex\", \"fragment\", \"compute\""
    ))),
  }
}

// ---------------------------------------------------------------------------
// Validation knobs. Names mirror naga exactly (lowercased); keep the tables,
// the `ts_type` unions below, and the drift tests in sync on naga upgrades.
// ---------------------------------------------------------------------------

const VALIDATION_FLAG_NAMES: &[&str] = &[
  "expressions",
  "blocks",
  "control_flow_uniformity",
  "struct_layouts",
  "constants",
  "bindings",
];

const CAPABILITY_NAMES: &[&str] = &[
  "immediates",
  "float64",
  "primitive_index",
  "texture_and_sampler_binding_array",
  "buffer_binding_array",
  "storage_texture_binding_array",
  "storage_buffer_binding_array",
  "clip_distances",
  "cull_distance",
  "storage_texture_16bit_norm_formats",
  "multiview",
  "early_depth_test",
  "multisampled_shading",
  "ray_query",
  "dual_source_blending",
  "cube_array_textures",
  "shader_int64",
  "subgroup",
  "subgroup_barrier",
  "subgroup_vertex_stage",
  "shader_int64_atomic_min_max",
  "shader_int64_atomic_all_ops",
  "shader_float32_atomic",
  "texture_atomic",
  "texture_int64_atomic",
  "ray_hit_vertex_position",
  "shader_float16",
  "texture_external",
  "shader_float16_in_float32",
  "shader_barycentrics",
  "mesh_shader",
  "mesh_shader_point_topology",
  "texture_and_sampler_binding_array_non_uniform_indexing",
  "buffer_binding_array_non_uniform_indexing",
  "storage_texture_binding_array_non_uniform_indexing",
  "storage_buffer_binding_array_non_uniform_indexing",
  "cooperative_matrix",
  "per_vertex",
  "ray_tracing_pipeline",
  "draw_index",
  "acceleration_structure_binding_array",
  "memory_decoration_coherent",
  "memory_decoration_volatile",
  "shader_int16",
];

const SUBGROUP_STAGE_NAMES: &[&str] = &[
  "vertex",
  "fragment",
  "compute",
  "mesh",
  "task",
  "ray_generation",
  "any_hit",
  "closest_hit",
  "miss",
];

const SUBGROUP_OPERATION_NAMES: &[&str] = &[
  "basic",
  "vote",
  "arithmetic",
  "ballot",
  "shuffle",
  "shuffle_relative",
  "quad_fragment_compute",
];

fn parse_validation_flags(list: &[String]) -> Result<naga::valid::ValidationFlags, Diagnostic> {
  use naga::valid::ValidationFlags as F;
  parse_flag_list(
    list,
    F::all(),
    F::empty(),
    "flag",
    VALIDATION_FLAG_NAMES,
    |name| match name {
      "expressions" => Some(F::EXPRESSIONS),
      "blocks" => Some(F::BLOCKS),
      "control_flow_uniformity" => Some(F::CONTROL_FLOW_UNIFORMITY),
      "struct_layouts" => Some(F::STRUCT_LAYOUTS),
      "constants" => Some(F::CONSTANTS),
      "bindings" => Some(F::BINDINGS),
      _ => None,
    },
  )
}

fn parse_capabilities(list: &[String]) -> Result<naga::valid::Capabilities, Diagnostic> {
  use naga::valid::Capabilities as C;
  parse_flag_list(
    list,
    C::all(),
    C::empty(),
    "capability",
    CAPABILITY_NAMES,
    |name| match name {
      "immediates" => Some(C::IMMEDIATES),
      "float64" => Some(C::FLOAT64),
      "primitive_index" => Some(C::PRIMITIVE_INDEX),
      "texture_and_sampler_binding_array" => Some(C::TEXTURE_AND_SAMPLER_BINDING_ARRAY),
      "buffer_binding_array" => Some(C::BUFFER_BINDING_ARRAY),
      "storage_texture_binding_array" => Some(C::STORAGE_TEXTURE_BINDING_ARRAY),
      "storage_buffer_binding_array" => Some(C::STORAGE_BUFFER_BINDING_ARRAY),
      "clip_distances" => Some(C::CLIP_DISTANCES),
      "cull_distance" => Some(C::CULL_DISTANCE),
      "storage_texture_16bit_norm_formats" => Some(C::STORAGE_TEXTURE_16BIT_NORM_FORMATS),
      "multiview" => Some(C::MULTIVIEW),
      "early_depth_test" => Some(C::EARLY_DEPTH_TEST),
      "multisampled_shading" => Some(C::MULTISAMPLED_SHADING),
      "ray_query" => Some(C::RAY_QUERY),
      "dual_source_blending" => Some(C::DUAL_SOURCE_BLENDING),
      "cube_array_textures" => Some(C::CUBE_ARRAY_TEXTURES),
      "shader_int64" => Some(C::SHADER_INT64),
      "subgroup" => Some(C::SUBGROUP),
      "subgroup_barrier" => Some(C::SUBGROUP_BARRIER),
      "subgroup_vertex_stage" => Some(C::SUBGROUP_VERTEX_STAGE),
      "shader_int64_atomic_min_max" => Some(C::SHADER_INT64_ATOMIC_MIN_MAX),
      "shader_int64_atomic_all_ops" => Some(C::SHADER_INT64_ATOMIC_ALL_OPS),
      "shader_float32_atomic" => Some(C::SHADER_FLOAT32_ATOMIC),
      "texture_atomic" => Some(C::TEXTURE_ATOMIC),
      "texture_int64_atomic" => Some(C::TEXTURE_INT64_ATOMIC),
      "ray_hit_vertex_position" => Some(C::RAY_HIT_VERTEX_POSITION),
      "shader_float16" => Some(C::SHADER_FLOAT16),
      "texture_external" => Some(C::TEXTURE_EXTERNAL),
      "shader_float16_in_float32" => Some(C::SHADER_FLOAT16_IN_FLOAT32),
      "shader_barycentrics" => Some(C::SHADER_BARYCENTRICS),
      "mesh_shader" => Some(C::MESH_SHADER),
      "mesh_shader_point_topology" => Some(C::MESH_SHADER_POINT_TOPOLOGY),
      "texture_and_sampler_binding_array_non_uniform_indexing" => {
        Some(C::TEXTURE_AND_SAMPLER_BINDING_ARRAY_NON_UNIFORM_INDEXING)
      }
      "buffer_binding_array_non_uniform_indexing" => {
        Some(C::BUFFER_BINDING_ARRAY_NON_UNIFORM_INDEXING)
      }
      "storage_texture_binding_array_non_uniform_indexing" => {
        Some(C::STORAGE_TEXTURE_BINDING_ARRAY_NON_UNIFORM_INDEXING)
      }
      "storage_buffer_binding_array_non_uniform_indexing" => {
        Some(C::STORAGE_BUFFER_BINDING_ARRAY_NON_UNIFORM_INDEXING)
      }
      "cooperative_matrix" => Some(C::COOPERATIVE_MATRIX),
      "per_vertex" => Some(C::PER_VERTEX),
      "ray_tracing_pipeline" => Some(C::RAY_TRACING_PIPELINE),
      "draw_index" => Some(C::DRAW_INDEX),
      "acceleration_structure_binding_array" => Some(C::ACCELERATION_STRUCTURE_BINDING_ARRAY),
      "memory_decoration_coherent" => Some(C::MEMORY_DECORATION_COHERENT),
      "memory_decoration_volatile" => Some(C::MEMORY_DECORATION_VOLATILE),
      "shader_int16" => Some(C::SHADER_INT16),
      _ => None,
    },
  )
}

fn parse_subgroup_stages(list: &[String]) -> Result<naga::valid::ShaderStages, Diagnostic> {
  use naga::valid::ShaderStages as S;
  parse_flag_list(
    list,
    S::all(),
    S::empty(),
    "subgroup stage",
    SUBGROUP_STAGE_NAMES,
    |name| match name {
      "vertex" => Some(S::VERTEX),
      "fragment" => Some(S::FRAGMENT),
      "compute" => Some(S::COMPUTE),
      "mesh" => Some(S::MESH),
      "task" => Some(S::TASK),
      "ray_generation" => Some(S::RAY_GENERATION),
      "any_hit" => Some(S::ANY_HIT),
      "closest_hit" => Some(S::CLOSEST_HIT),
      "miss" => Some(S::MISS),
      _ => None,
    },
  )
}

fn parse_subgroup_operations(
  list: &[String],
) -> Result<naga::valid::SubgroupOperationSet, Diagnostic> {
  use naga::valid::SubgroupOperationSet as O;
  parse_flag_list(
    list,
    O::all(),
    O::empty(),
    "subgroup operation",
    SUBGROUP_OPERATION_NAMES,
    |name| match name {
      "basic" => Some(O::BASIC),
      "vote" => Some(O::VOTE),
      "arithmetic" => Some(O::ARITHMETIC),
      "ballot" => Some(O::BALLOT),
      "shuffle" => Some(O::SHUFFLE),
      "shuffle_relative" => Some(O::SHUFFLE_RELATIVE),
      "quad_fragment_compute" => Some(O::QUAD_FRAGMENT_COMPUTE),
      _ => None,
    },
  )
}

fn parse_flag_list<A>(
  list: &[String],
  all: A,
  empty: A,
  what: &'static str,
  valid: &[&str],
  mut parse: impl FnMut(&str) -> Option<A>,
) -> Result<A, Diagnostic>
where
  A: Copy + std::ops::BitOr<Output = A> + std::ops::BitOrAssign,
{
  if list.iter().any(|name| name == "all") {
    return Ok(all);
  }
  let mut out = empty;
  for name in list {
    match parse(name) {
      Some(flag) => out |= flag,
      None => {
        let mut expected = String::from("\"all\"");
        for candidate in valid {
          expected.push_str(", \"");
          expected.push_str(candidate);
          expected.push('"');
        }
        return Err(Diagnostic::usage(format!(
          "invalid {what} '{name}'; expected one of {expected}"
        )));
      }
    }
  }
  Ok(out)
}

struct ValidatorConfig {
  flags: naga::valid::ValidationFlags,
  capabilities: naga::valid::Capabilities,
  stages: naga::valid::ShaderStages,
  operations: naga::valid::SubgroupOperationSet,
}

fn validator_config(validation: Option<&ValidationOptions>) -> Result<ValidatorConfig, Diagnostic> {
  let empty = ValidationOptions {
    flags: None,
    capabilities: None,
    subgroup_stages: None,
    subgroup_operations: None,
  };
  let validation = validation.unwrap_or(&empty);
  Ok(ValidatorConfig {
    flags: match &validation.flags {
      Some(list) => parse_validation_flags(list)?,
      None => naga::valid::ValidationFlags::all(),
    },
    capabilities: match &validation.capabilities {
      Some(list) => parse_capabilities(list)?,
      None => naga::valid::Capabilities::all(),
    },
    stages: match &validation.subgroup_stages {
      Some(list) => parse_subgroup_stages(list)?,
      None => naga::valid::ShaderStages::all(),
    },
    operations: match &validation.subgroup_operations {
      Some(list) => parse_subgroup_operations(list)?,
      None => naga::valid::SubgroupOperationSet::all(),
    },
  })
}

/// Validator strictness. Every field is optional and defaults to `"all"`;
/// pass `"all"` explicitly inside an array for the same effect, or `[] for
/// none. Names mirror naga exactly (lowercased).
#[napi(object)]
pub struct ValidationOptions {
  /// Which validation passes run (`expressions`, `blocks`,
  /// `control_flow_uniformity`, `struct_layouts`, `constants`, `bindings`).
  #[napi(
    ts_type = "(\"all\" | \"expressions\" | \"blocks\" | \"control_flow_uniformity\" | \"struct_layouts\" | \"constants\" | \"bindings\")[]"
  )]
  pub flags: Option<Vec<String>>,
  /// Allowed IR capabilities, e.g. `"shader_float16"`, `"subgroup"`,
  /// `"mesh_shader"`. Restrict these to validate against what a target
  /// platform actually accepts instead of everything naga can express.
  #[napi(
    ts_type = "(\"all\" | \"immediates\" | \"float64\" | \"primitive_index\" | \"texture_and_sampler_binding_array\" | \"buffer_binding_array\" | \"storage_texture_binding_array\" | \"storage_buffer_binding_array\" | \"clip_distances\" | \"cull_distance\" | \"storage_texture_16bit_norm_formats\" | \"multiview\" | \"early_depth_test\" | \"multisampled_shading\" | \"ray_query\" | \"dual_source_blending\" | \"cube_array_textures\" | \"shader_int64\" | \"subgroup\" | \"subgroup_barrier\" | \"subgroup_vertex_stage\" | \"shader_int64_atomic_min_max\" | \"shader_int64_atomic_all_ops\" | \"shader_float32_atomic\" | \"texture_atomic\" | \"texture_int64_atomic\" | \"ray_hit_vertex_position\" | \"shader_float16\" | \"texture_external\" | \"shader_float16_in_float32\" | \"shader_barycentrics\" | \"mesh_shader\" | \"mesh_shader_point_topology\" | \"texture_and_sampler_binding_array_non_uniform_indexing\" | \"buffer_binding_array_non_uniform_indexing\" | \"storage_texture_binding_array_non_uniform_indexing\" | \"storage_buffer_binding_array_non_uniform_indexing\" | \"cooperative_matrix\" | \"per_vertex\" | \"ray_tracing_pipeline\" | \"draw_index\" | \"acceleration_structure_binding_array\" | \"memory_decoration_coherent\" | \"memory_decoration_volatile\" | \"shader_int16\")[]"
  )]
  pub capabilities: Option<Vec<String>>,
  /// Stages where subgroup operations are permitted.
  #[napi(
    ts_type = "(\"all\" | \"vertex\" | \"fragment\" | \"compute\" | \"mesh\" | \"task\" | \"ray_generation\" | \"any_hit\" | \"closest_hit\" | \"miss\")[]"
  )]
  pub subgroup_stages: Option<Vec<String>>,
  /// Permitted subgroup operations.
  #[napi(
    ts_type = "(\"all\" | \"basic\" | \"vote\" | \"arithmetic\" | \"ballot\" | \"shuffle\" | \"shuffle_relative\" | \"quad_fragment_compute\")[]"
  )]
  pub subgroup_operations: Option<Vec<String>>,
}

/// Options for [`translate`].
#[napi(object)]
pub struct TranslateOptions {
  /// Input language of `source`.
  #[napi(ts_type = "\"wgsl\" | \"glsl\" | \"spv\"")]
  pub from: String,
  /// Desired output language.
  #[napi(ts_type = "\"wgsl\" | \"glsl\" | \"hlsl\" | \"msl\" | \"spv\" | \"dot\"")]
  pub to: String,
  /// Shader source: text for `wgsl`/`glsl`, raw SPIR-V bytes for `spv`.
  pub source: Either<String, Uint8Array>,
  /// Entry-point stage. Required when the module has more than one entry
  /// point and the backend needs a single one (`glsl`, `spv`); otherwise the
  /// sole entry point is used automatically.
  #[napi(ts_type = "\"vertex\" | \"fragment\" | \"compute\"")]
  pub stage: Option<String>,
  /// Entry-point name. Same resolution rules as `stage`.
  pub entry_point: Option<String>,
  /// GLSL output version, e.g. `"310 es"` (default) or `"450"`.
  pub glsl_version: Option<String>,
  /// HLSL shader model, e.g. `"5_1"` (default), `"6_0"`.
  pub hlsl_shader_model: Option<String>,
  /// Preprocessor definitions for GLSL *input* (`#define key value` pairs).
  pub defines: Option<HashMap<String, String>>,
  /// Validator strictness. Defaults to everything enabled.
  pub validation: Option<ValidationOptions>,
}

/// One source location attached to a diagnostic.
#[napi(object)]
pub struct NagaLabel {
  /// 1-based line number.
  pub line: u32,
  /// 1-based column (bytes) of the start of the span.
  pub column: u32,
  /// Length (bytes) of the span.
  pub length: u32,
  /// What this location refers to.
  pub message: String,
}

/// Machine-readable diagnostic. `message` is the human-oriented rendering
/// (stable: identical to what the throwing API puts in the `Error`);
/// `labels` is the machine interface for editors and tooling.
#[napi(object)]
pub struct NagaDiagnostic {
  /// Error class: what stage of the pipeline failed.
  #[napi(ts_type = "\"parse\" | \"validation\" | \"backend\" | \"usage\"")]
  pub kind: String,
  /// Human-readable rendering of the error.
  pub message: String,
  /// Source locations involved, empty when the error carries no spans
  /// (e.g. backend errors or SPIR-V input).
  pub labels: Vec<NagaLabel>,
  /// Extra context lines from the frontend, if any.
  pub notes: Vec<String>,
}

/// Result of [`validate_wgsl_detailed`].
#[napi(object)]
pub struct ValidationResult {
  pub ok: bool,
  pub error: Option<NagaDiagnostic>,
}

/// Result of [`translate_detailed`].
#[napi(object)]
pub struct TranslateResult {
  pub ok: bool,
  /// Present when `ok` is true: text, except `Uint8Array` for `spv` output.
  pub output: Option<Either<String, Uint8Array>>,
  /// Present when `ok` is false.
  pub error: Option<NagaDiagnostic>,
}

// ---------------------------------------------------------------------------
// JS-facing functions
// ---------------------------------------------------------------------------

/// Parse and validate a WGSL shader. Throws on error.
#[napi]
pub fn validate_wgsl(source: String, validation: Option<ValidationOptions>) -> napi::Result<()> {
  parse_and_validate(
    InputFormat::Wgsl,
    source.as_bytes(),
    None,
    None,
    validation.as_ref(),
  )
  .map(|_| ())
  .map_err(to_napi_error)
}

/// Parse and validate a WGSL shader without throwing.
///
/// Returns `{ ok: true }` on success or `{ ok: false, error }` with a
/// structured [`NagaDiagnostic`] on failure.
#[napi]
pub fn validate_wgsl_detailed(
  source: String,
  validation: Option<ValidationOptions>,
) -> ValidationResult {
  match parse_and_validate(
    InputFormat::Wgsl,
    source.as_bytes(),
    None,
    None,
    validation.as_ref(),
  ) {
    Ok(_) => ValidationResult {
      ok: true,
      error: None,
    },
    Err(diagnostic) => ValidationResult {
      ok: false,
      error: Some(diagnostic.into_napi()),
    },
  }
}

/// Version of the wrapped `naga` crate (e.g. `"30.0.1"`).
///
/// This reports the upstream recorded in the package.json `naga` field, which
/// the package version tracks (see `scripts/sync-version.mjs`).
#[napi]
pub fn naga_version() -> String {
  env!("NAGA_VERSION").to_owned()
}

/// Translate a shader from one language to another.
///
/// Returns text for every output format except `spv`, which returns the raw
/// SPIR-V word stream as a `Uint8Array` (little-endian bytes). Throws on
/// parse, validation, or backend errors.
#[napi]
pub fn translate(options: TranslateOptions) -> napi::Result<Either<String, Uint8Array>> {
  translate_impl(options).map_err(to_napi_error)
}

/// Translate a shader without throwing.
///
/// Returns `{ ok: true, output }` on success or `{ ok: false, error }` with a
/// structured [`NagaDiagnostic`] on failure.
#[napi]
pub fn translate_detailed(options: TranslateOptions) -> TranslateResult {
  match translate_impl(options) {
    Ok(output) => TranslateResult {
      ok: true,
      output: Some(output),
      error: None,
    },
    Err(diagnostic) => TranslateResult {
      ok: false,
      output: None,
      error: Some(diagnostic.into_napi()),
    },
  }
}

// ---------------------------------------------------------------------------
// Core pipeline (plain Rust: no napi types except at the boundary)
// ---------------------------------------------------------------------------

/// Internal failure value. `message` is the human-oriented body; the throwing
/// API prefixes it by kind (`to_napi_error`), while the detailed API also
/// exposes `labels`/`notes` as data. Keep thrown strings stable: existing
/// tests assert their shape.
#[derive(Debug)]
struct Diagnostic {
  kind: &'static str,
  /// Display name of the stage for thrown prefixes, e.g. `"WGSL"`, `"HLSL"`.
  /// Empty for kinds without a stage (`validation`, `usage`).
  form: &'static str,
  message: String,
  labels: Vec<Label>,
  notes: Vec<String>,
}

impl Diagnostic {
  fn parse(form: &'static str, message: String) -> Self {
    Diagnostic {
      kind: "parse",
      form,
      message,
      labels: Vec::new(),
      notes: Vec::new(),
    }
  }

  fn usage(message: String) -> Self {
    Diagnostic {
      kind: "usage",
      form: "",
      message,
      labels: Vec::new(),
      notes: Vec::new(),
    }
  }

  fn into_napi(self) -> NagaDiagnostic {
    NagaDiagnostic {
      kind: self.kind.to_owned(),
      message: self.message,
      labels: self
        .labels
        .into_iter()
        .map(|label| NagaLabel {
          line: label.line,
          column: label.column,
          length: label.length,
          message: label.message,
        })
        .collect(),
      notes: self.notes,
    }
  }
}

/// Internal label before crossing into a `#[napi(object)]`.
#[derive(Debug)]
struct Label {
  line: u32,
  column: u32,
  length: u32,
  message: String,
}

/// Attach a source span to `labels` when it points into text source.
fn push_label(labels: &mut Vec<Label>, span: naga::Span, message: &str, text: Option<&str>) {
  if !span.is_defined() {
    return;
  }
  if let Some(source) = text {
    let loc = span.location(source);
    labels.push(Label {
      line: loc.line_number,
      column: loc.line_position,
      length: loc.length,
      message: message.to_owned(),
    });
  }
}

fn to_napi_error(d: Diagnostic) -> napi::Error {
  let prefix = match d.kind {
    "parse" => format!("{} parse error:\n", d.form),
    "validation" => "validation error:\n".to_owned(),
    "backend" => format!("{} backend error: ", d.form),
    _ => String::new(),
  };
  napi::Error::new(
    napi::Status::GenericFailure,
    format!("{prefix}{}", d.message),
  )
}

struct Parsed {
  module: naga::Module,
}

fn parse_and_validate(
  from: InputFormat,
  source: &[u8],
  stage: Option<naga::ShaderStage>,
  defines: Option<&HashMap<String, String>>,
  validation: Option<&ValidationOptions>,
) -> Result<(Parsed, naga::valid::ModuleInfo), Diagnostic> {
  let (module, text) = match from {
    InputFormat::Wgsl => {
      let text = std::str::from_utf8(source)
        .map_err(|e| Diagnostic::parse("WGSL", format!("source is not valid UTF-8: {e}")))?
        .to_owned();
      let module = naga::front::wgsl::parse_str(&text).map_err(|e| {
        let mut labels = Vec::new();
        for (span, message) in e.labels() {
          push_label(&mut labels, span, message, Some(&text));
        }
        Diagnostic {
          kind: "parse",
          form: "WGSL",
          message: e.emit_to_string(&text),
          labels,
          notes: e.notes().map(str::to_owned).collect(),
        }
      })?;
      (module, Some(text))
    }
    InputFormat::Glsl => {
      let text = std::str::from_utf8(source)
        .map_err(|e| Diagnostic::parse("GLSL", format!("source is not valid UTF-8: {e}")))?
        .to_owned();
      let stage = stage.ok_or_else(|| {
        Diagnostic::usage("GLSL input requires `stage` (vertex, fragment, or compute)".to_owned())
      })?;
      let mut frontend = naga::front::glsl::Frontend::default();
      let mut options = naga::front::glsl::Options::from(stage);
      if let Some(defines) = defines {
        options.defines = defines
          .iter()
          .map(|(k, v)| (k.clone(), v.clone()))
          .collect();
      }
      let module = frontend.parse(&options, &text).map_err(|e| {
        let mut labels = Vec::new();
        for err in &e.errors {
          push_label(&mut labels, err.meta, &err.kind.to_string(), Some(&text));
        }
        Diagnostic {
          kind: "parse",
          form: "GLSL",
          message: e.emit_to_string(&text),
          labels,
          notes: Vec::new(),
        }
      })?;
      (module, Some(text))
    }
    InputFormat::Spv => {
      let module = naga::front::spv::parse_u8_slice(source, &naga::front::spv::Options::default())
        .map_err(|e| Diagnostic::parse("SPIR-V", e.to_string()))?;
      (module, None)
    }
  };

  let config = validator_config(validation)?;
  let info = naga::valid::Validator::new(config.flags, config.capabilities)
    .subgroup_stages(config.stages)
    .subgroup_operations(config.operations)
    .validate(&module)
    .map_err(|e| {
      let mut labels = Vec::new();
      for (span, message) in e.spans() {
        push_label(&mut labels, *span, message, text.as_deref());
      }
      Diagnostic {
        kind: "validation",
        form: "",
        message: with_location(&e, text.as_deref()),
        labels,
        notes: Vec::new(),
      }
    })?;

  Ok((Parsed { module }, info))
}

/// Human-readable validation message, with `line:column` when the error
/// carries a span into text source.
fn with_location(e: &naga::WithSpan<naga::valid::ValidationError>, text: Option<&str>) -> String {
  let base = e.to_string();
  match text {
    Some(source) => match e.location(source) {
      Some(loc) => format!(
        "{base} (at line {}, column {})",
        loc.line_number, loc.line_position
      ),
      None => base,
    },
    None => base,
  }
}

fn resolve_entry_point(
  module: &naga::Module,
  stage: Option<naga::ShaderStage>,
  name: Option<&str>,
) -> Result<(naga::ShaderStage, String), Diagnostic> {
  if let (Some(stage), Some(name)) = (stage, name) {
    if module
      .entry_points
      .iter()
      .any(|ep| ep.stage == stage && ep.name == name)
    {
      return Ok((stage, name.to_owned()));
    }
    return Err(Diagnostic::usage(format!(
      "entry point '{name}' with stage {stage:?} not found in module"
    )));
  }
  if module.entry_points.len() == 1 {
    let ep = module.entry_points.first().expect("len checked");
    return Ok((ep.stage, ep.name.clone()));
  }
  Err(Diagnostic::usage(
    "shader has multiple entry points; specify `stage` and `entryPoint`".to_owned(),
  ))
}

fn parse_glsl_version(spec: &str) -> Result<naga::back::glsl::Version, Diagnostic> {
  use naga::back::glsl::Version;
  let trimmed = spec.trim().to_ascii_lowercase();
  let (number, es) = match trimmed.strip_suffix("es") {
    Some(number) => (number.trim(), true),
    None => (trimmed.as_str(), false),
  };
  let version: u16 = number.parse().map_err(|_| {
    Diagnostic::usage(format!(
      "invalid `glslVersion` '{spec}'; expected e.g. \"310 es\" or \"450\""
    ))
  })?;
  Ok(if es {
    Version::new_gles(version)
  } else {
    Version::Desktop(version)
  })
}

fn parse_hlsl_shader_model(spec: &str) -> Result<naga::back::hlsl::ShaderModel, Diagnostic> {
  use naga::back::hlsl::ShaderModel;
  match spec.trim() {
    "5_0" => Ok(ShaderModel::V5_0),
    "5_1" => Ok(ShaderModel::V5_1),
    "6_0" => Ok(ShaderModel::V6_0),
    "6_1" => Ok(ShaderModel::V6_1),
    "6_2" => Ok(ShaderModel::V6_2),
    "6_3" => Ok(ShaderModel::V6_3),
    "6_4" => Ok(ShaderModel::V6_4),
    "6_5" => Ok(ShaderModel::V6_5),
    "6_6" => Ok(ShaderModel::V6_6),
    "6_7" => Ok(ShaderModel::V6_7),
    "6_8" => Ok(ShaderModel::V6_8),
    "6_9" => Ok(ShaderModel::V6_9),
    other => Err(Diagnostic::usage(format!(
      "invalid `hlslShaderModel` '{other}'; expected e.g. \"5_1\" or \"6_0\""
    ))),
  }
}

fn translate_impl(options: TranslateOptions) -> Result<Either<String, Uint8Array>, Diagnostic> {
  let from = parse_input_format(&options.from)?;
  let to = parse_output_format(&options.to)?;
  let source_bytes: Vec<u8> = match options.source {
    Either::A(text) => text.into_bytes(),
    Either::B(bytes) => bytes.to_vec(),
  };
  let stage = options
    .stage
    .as_deref()
    .map(parse_shader_stage)
    .transpose()?
    .map(|stage| match stage {
      ShaderStage::Vertex => naga::ShaderStage::Vertex,
      ShaderStage::Fragment => naga::ShaderStage::Fragment,
      ShaderStage::Compute => naga::ShaderStage::Compute,
    });

  let (parsed, info) = parse_and_validate(
    from,
    &source_bytes,
    stage,
    options.defines.as_ref(),
    options.validation.as_ref(),
  )?;
  let module = &parsed.module;

  match to {
    OutputFormat::Wgsl => {
      let out =
        naga::back::wgsl::write_string(module, &info, naga::back::wgsl::WriterFlags::empty())
          .map_err(|e| Diagnostic {
            kind: "backend",
            form: "WGSL",
            message: e.to_string(),
            labels: Vec::new(),
            notes: Vec::new(),
          })?;
      Ok(Either::A(out))
    }
    OutputFormat::Glsl => {
      let (ep_stage, ep_name) = resolve_entry_point(module, stage, options.entry_point.as_deref())?;
      let version = match options.glsl_version.as_deref() {
        Some(spec) => parse_glsl_version(spec)?,
        None => naga::back::glsl::Version::new_gles(310),
      };
      let glsl_options = naga::back::glsl::Options {
        version,
        ..Default::default()
      };
      let pipeline_options = naga::back::glsl::PipelineOptions {
        shader_stage: ep_stage,
        entry_point: ep_name,
        multiview: None,
      };
      let mut out = String::new();
      naga::back::glsl::Writer::new(
        &mut out,
        module,
        &info,
        &glsl_options,
        &pipeline_options,
        naga::proc::BoundsCheckPolicies::default(),
      )
      .and_then(|mut writer| writer.write())
      .map_err(|e| Diagnostic {
        kind: "backend",
        form: "GLSL",
        message: e.to_string(),
        labels: Vec::new(),
        notes: Vec::new(),
      })?;
      Ok(Either::A(out))
    }
    OutputFormat::Hlsl => {
      let shader_model = match options.hlsl_shader_model.as_deref() {
        Some(spec) => parse_hlsl_shader_model(spec)?,
        None => naga::back::hlsl::ShaderModel::V5_1,
      };
      let hlsl_options = naga::back::hlsl::Options {
        shader_model,
        ..Default::default()
      };
      let entry_point = match (&stage, &options.entry_point) {
        (Some(stage), Some(name)) => Some((*stage, name.clone())),
        _ if module.entry_points.len() == 1 => {
          let ep = module.entry_points.first().expect("len checked");
          Some((ep.stage, ep.name.clone()))
        }
        _ => None,
      };
      let pipeline_options = naga::back::hlsl::PipelineOptions { entry_point };
      // Fragment entry points need I/O linking info for correct HLSL.
      let fragment = match &pipeline_options.entry_point {
        Some((stage, name)) if *stage == naga::ShaderStage::Fragment => {
          naga::back::hlsl::FragmentEntryPoint::new(module, name)
        }
        _ => None,
      };
      let mut out = String::new();
      naga::back::hlsl::Writer::new(&mut out, &hlsl_options, &pipeline_options)
        .write(module, &info, fragment.as_ref())
        .map_err(|e| Diagnostic {
          kind: "backend",
          form: "HLSL",
          message: e.to_string(),
          labels: Vec::new(),
          notes: Vec::new(),
        })?;
      Ok(Either::A(out))
    }
    OutputFormat::Msl => {
      let entry_point = match (&stage, &options.entry_point) {
        (Some(stage), Some(name)) => Some((*stage, name.clone())),
        _ if module.entry_points.len() == 1 => {
          let ep = module.entry_points.first().expect("len checked");
          Some((ep.stage, ep.name.clone()))
        }
        _ => None,
      };
      let pipeline_options = naga::back::msl::PipelineOptions {
        entry_point,
        ..Default::default()
      };
      let (out, _translation_info) = naga::back::msl::write_string(
        module,
        &info,
        &naga::back::msl::Options::default(),
        &pipeline_options,
      )
      .map_err(|e| Diagnostic {
        kind: "backend",
        form: "MSL",
        message: e.to_string(),
        labels: Vec::new(),
        notes: Vec::new(),
      })?;
      Ok(Either::A(out))
    }
    OutputFormat::Spv => {
      let pipeline_options =
        resolve_entry_point_optional(module, stage, options.entry_point.as_deref())?.map(
          |(ep_stage, ep_name)| naga::back::spv::PipelineOptions {
            shader_stage: ep_stage,
            entry_point: ep_name,
          },
        );
      let words = naga::back::spv::write_vec(
        module,
        &info,
        &naga::back::spv::Options::default(),
        pipeline_options.as_ref(),
      )
      .map_err(|e| Diagnostic {
        kind: "backend",
        form: "SPIR-V",
        message: e.to_string(),
        labels: Vec::new(),
        notes: Vec::new(),
      })?;
      let mut bytes = Vec::with_capacity(words.len() * 4);
      for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
      }
      Ok(Either::B(Uint8Array::new(bytes)))
    }
    OutputFormat::Dot => {
      let out = naga::back::dot::write(module, Some(&info), naga::back::dot::Options::default())
        .map_err(|e| Diagnostic {
          kind: "backend",
          form: "DOT",
          message: e.to_string(),
          labels: Vec::new(),
          notes: Vec::new(),
        })?;
      Ok(Either::A(out))
    }
  }
}

/// Like [`resolve_entry_point`], but returns `None` (translate all entry
/// points) instead of erroring when the caller selected nothing and the
/// backend supports multi-entry-point output.
fn resolve_entry_point_optional(
  module: &naga::Module,
  stage: Option<naga::ShaderStage>,
  name: Option<&str>,
) -> Result<Option<(naga::ShaderStage, String)>, Diagnostic> {
  match (stage, name) {
    (None, None) => {
      if module.entry_points.len() == 1 {
        let ep = module.entry_points.first().expect("len checked");
        return Ok(Some((ep.stage, ep.name.clone())));
      }
      Ok(None)
    }
    _ => resolve_entry_point(module, stage, name).map(Some),
  }
}

#[cfg(test)]
mod drift_tests {
  use super::*;

  /// Every name in our tables must parse, and together they must cover the
  /// whole bitflag set. Fails the moment naga adds (or renames) a variant,
  /// pointing at the table and `ts_type` union to update.
  fn assert_covers_all<A>(names: &[&str], parse: impl Fn(&str) -> Option<A>, all: A)
  where
    A: Copy + std::ops::BitOr<Output = A> + PartialEq + std::fmt::Debug,
  {
    let mut parsed = names
      .iter()
      .map(|name| parse(name).unwrap_or_else(|| panic!("stale validation name in table: {name}")));
    let first = parsed.next().expect("table must not be empty");
    assert_eq!(parsed.fold(first, |a, b| a | b), all);
  }

  #[test]
  fn validation_flag_names_cover_naga() {
    assert_covers_all(
      VALIDATION_FLAG_NAMES,
      |name| {
        parse_validation_flags(&[name.to_owned()])
          .ok()
          .filter(|flags| !flags.is_empty())
      },
      naga::valid::ValidationFlags::all(),
    );
  }

  #[test]
  fn capability_names_cover_naga() {
    assert_covers_all(
      CAPABILITY_NAMES,
      |name| {
        parse_capabilities(&[name.to_owned()])
          .ok()
          .filter(|caps| !caps.is_empty())
      },
      naga::valid::Capabilities::all(),
    );
  }

  #[test]
  fn subgroup_stage_names_cover_naga() {
    assert_covers_all(
      SUBGROUP_STAGE_NAMES,
      |name| {
        parse_subgroup_stages(&[name.to_owned()])
          .ok()
          .filter(|stages| !stages.is_empty())
      },
      naga::valid::ShaderStages::all(),
    );
  }

  #[test]
  fn subgroup_operation_names_cover_naga() {
    assert_covers_all(
      SUBGROUP_OPERATION_NAMES,
      |name| {
        parse_subgroup_operations(&[name.to_owned()])
          .ok()
          .filter(|ops| !ops.is_empty())
      },
      naga::valid::SubgroupOperationSet::all(),
    );
  }

  #[test]
  fn unknown_validation_names_are_rejected() {
    assert!(parse_validation_flags(&["bogus".to_owned()]).is_err());
    assert!(parse_capabilities(&["nope".to_owned()]).is_err());
    assert!(parse_subgroup_stages(&["bogus".to_owned()]).is_err());
    assert!(parse_subgroup_operations(&["nope".to_owned()]).is_err());
  }

  #[test]
  fn explicit_all_selects_everything() {
    assert_eq!(
      parse_validation_flags(&["all".to_owned()]).expect("all"),
      naga::valid::ValidationFlags::all()
    );
    assert_eq!(
      parse_capabilities(&["all".to_owned()]).expect("all"),
      naga::valid::Capabilities::all()
    );
  }

  #[test]
  fn golden_wgsl_to_glsl_without_node() {
    let options = TranslateOptions {
      from: "wgsl".to_owned(),
      to: "glsl".to_owned(),
      source: Either::A(
        "\n@fragment\nfn main_fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(1.0, 1.0, 1.0, 1.0);\n}\n".to_owned(),
      ),
      stage: Some("fragment".to_owned()),
      entry_point: Some("main_fs".to_owned()),
      glsl_version: None,
      hlsl_shader_model: None,
      defines: None,
      validation: None,
    };
    match translate_impl(options).expect("golden translation") {
      Either::A(glsl) => assert!(glsl.contains("_fs2p_location0"), "unexpected GLSL:\n{glsl}"),
      Either::B(_) => panic!("expected text output"),
    }
  }
}
