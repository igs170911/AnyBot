def reldir($root):
  .manifest_path | ltrimstr($root + "/") | rtrimstr("/Cargo.toml");

def has_prefix($prefixes):
  . as $path | any($prefixes[]; . as $p | $path | startswith($p));

def has_suffix($suffixes):
  . as $path | any($suffixes[]; . as $s | $path | endswith($s));

def basename:
  split("/") | last;

def dependents($pkgs; $names; $kinds):
  reduce (
    $pkgs[] | . as $p | .deps[]
    | select(.name as $n | any($names[]; . == $n))
    | select(.kind as $k | any($kinds[]; . == $k))
    | {from: $p.name, to: .name}
  ) as $e ({}; .[$e.to] += [$e.from]);

def closure($edges):
  {seen: ., frontier: .}
  | until(
      .frontier | length == 0;
      ([.frontier[] | $edges[.] // [] | .[]] | unique) as $next
      | ($next - .seen) as $fresh
      | {seen: ((.seen + $fresh) | unique), frontier: $fresh}
    )
  | .seen;

def owner($pkgs):
  . as $file
  | [$pkgs[] | . as $p | select($file | startswith($p.dir + "/"))]
  | sort_by(.dir | length)
  | last
  | if . == null then null else .name end;

def hit($files; f):
  [$files[] | select(f)] | first;

def decide($name; $evidence; $skip_reason):
  if $evidence == null
  then {name: $name, run: false, reason: $skip_reason}
  else {name: $name, run: true, reason: $evidence}
  end;

def changed_because($file):
  if $file == null then null else "\($file) changed" end;

(
  $changed | split("\n") | map(select(length > 0)) | unique
) as $files
| .workspace_root as $root
| [
    .packages[]
    | {name: .name, dir: reldir($root), deps: [.dependencies[] | {name: .name, kind: .kind}]}
  ] as $pkgs
| ($pkgs | map(.name)) as $names
| ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "clippy.toml", "rustfmt.toml", ".config/nextest.toml"] as $global_files
| ($files | map(select(endswith(".md") | not))) as $code_files
| (
    $code_files
    | map({file: ., pkg: owner($pkgs)})
  ) as $owned
| (
    $files
    | any(. as $f | any($global_files[]; . == $f) or ($f | startswith(".cargo/")))
  ) as $global
| (
    if $global then $names
    else ([$owned[] | select(.pkg != null) | .pkg] | unique)
    end
  ) as $direct
| dependents($pkgs; $names; [null, "dev", "build"]) as $all_edges
| dependents($pkgs; $names; [null, "build"]) as $normal_edges
| ($direct | closure($all_edges)) as $affected
| ($direct | closure($normal_edges)) as $normal_affected
| [$pkgs[] | select(any(.deps[]; .name == "sqlx")) | .name] as $sqlx_packages
| (
    changed_because(hit($files; has_prefix(["migrations/", ".sqlx/"])))
    // (
      [$owned[] | select(.pkg as $p | any($sqlx_packages[]; . == $p)) | select(.file | endswith(".rs"))]
      | first
      | if . == null then null else "\(.file) changed in sqlx package \(.pkg)" end
    )
  ) as $sqlx_hit
| (
    changed_because(hit($files; has_prefix(["api/", "web/api-client/", "redocly"])))
    // (
      if any($normal_affected[]; . == "api-docs")
      then "package api-docs depends on a changed package"
      else null
      end
    )
  ) as $api_hit
| [
    {name: "fmt", run: true, reason: "always"},
    (
      if ($affected | length) == 0
      then {name: "clippy", run: false, reason: "no Rust package affected"}
      else {name: "clippy", run: true, reason: "\($affected | length) affected packages (\($direct | length) changed directly)"}
      end
    ),
    (
      if ($affected | length) == 0
      then {name: "test", run: false, reason: "no Rust package affected"}
      else {name: "test", run: true, reason: "\($affected | length) affected packages"}
      end
    ),
    decide(
      "depcheck";
      changed_because(hit($files; basename == "Cargo.toml" or basename == "Cargo.lock"));
      "no Cargo manifest or lock file changed"
    ),
    decide("sqlx-check"; $sqlx_hit; "no migrations or sqlx package source changed"),
    decide(
      "locale";
      changed_because(hit($files; has_prefix(["locales/", "crates/messages/", "web/ui/src/generated/"]) or has_suffix([".rs", ".ts", ".tsx"])));
      "no locale or source file changed"
    ),
    decide(
      "ironlaws";
      changed_because(hit($files; has_prefix(["Makefile", "docs/", "tools/", ".config/", "migrations/", ".github/workflows/", "deny.toml"]) or has_suffix([".rs", ".ts", ".tsx"]) or basename == "Cargo.toml" or basename == "clippy.toml"));
      "no source, tool, doc, or Makefile change"
    ),
    decide("api-docs"; $api_hit; "no API-related change"),
    decide("api-client"; $api_hit; "no API-related change"),
    decide("api-compat"; $api_hit; "no API-related change"),
    decide(
      "ts-biome";
      changed_because(hit($files; has_prefix(["web/", "tests/e2e/"]) or has_suffix([".ts", ".tsx"]) or . == "biome.json" or . == "package.json"));
      "no TypeScript or web change"
    ),
    decide(
      "check-changed-selftest";
      changed_because(hit($files; has_prefix(["tools/check-changed", "tests/check-changed-selftest/", "Makefile"])));
      "check-changed logic unchanged"
    )
  ] as $steps
| (["changed", ($files | length | tostring)] | @tsv),
  ($direct[] | ["direct", .] | @tsv),
  ($affected[] | . as $a | ["affected", $a, ($pkgs[] | select(.name == $a) | .dir)] | @tsv),
  ($steps[] | ["step", .name, (if .run then "run" else "skip" end), .reason] | @tsv)
