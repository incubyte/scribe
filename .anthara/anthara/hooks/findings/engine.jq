def SCHEMA: "anthara.findings/v1";
def REVIEW_AGENT_NAMES: ["reviewer", "pack-reviewer", "browser-verifier"];
def REVIEW_AGENT_TYPES: REVIEW_AGENT_NAMES + (REVIEW_AGENT_NAMES | map("anthara:" + .));
def SEVERITIES: ["high", "medium", "low"];
def TYPES: ["architecture", "integration", "code-quality", "test-quality", "ai-ergonomics", "accessibility", "compliance", "browser"];
def MODES: ["first-pass", "re-review", "re-verify"];
def OUTCOMES: ["closed", "still-failing", "unverified"];
def PASSING_OUTCOMES: ["closed", "unverified"];
def IN_FLIGHT_STATUSES: ["pending", "running", "paused"];
def NON_TERMINAL_STATUSES: ["open", "fixing"];
def TERMINAL_STATUSES: ["fixed", "deferred", "dropped"];
def REOPENABLE_STATUSES: ["claimed_fixed", "fixed"];
def FIELD_LENGTH_LIMITS: {file: 1024, symbol: 512, pack: 128, rule_title: 256, rule_id: 64};
def SOURCE_PATTERN: "reviewer|browser-verifier|pack-reviewer:[^\\n]{1,128}";
def UUID_PATTERN: "\\A[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\z";
def FIX_PREFIX: "[Anthara-Fix";
def DEFERRED_SUFFIX: "(deferred)";
def MAX_FINDINGS: 500;
def MAX_FINGERPRINT_LENGTH: 1024;
def MAX_WHAT_LENGTH: 200;
def MAX_BRIEFED_INCIDENTS: 100;
def HANDBACK_TOOL: "SubagentHandback";
def FORK_SCAN_LINES: 200;
def INCIDENT_REQUIRED_KEYS: ["id", "fingerprint", "severity", "type", "what", "file", "symbol", "first_seen_at", "status_changed_at"];
def LEDGER_REQUIRED_KEYS: ["schema", "run_id", "state", "started_at", "closed_at", "incidents", "unparsed", "orphan_todos", "totals"];
def BRIEF_HEADER: "Anthara findings already recorded in this run (id [sources] severity type file:symbol — what):";
def BRIEF_INSTRUCTION: "In your findings block, set a finding's \"incident\" to the id above that it restates, even if you now word its file or symbol differently; set \"incident\" to null for a new finding. In mode=re-review, every incident from your own source that you do not report is read as cleared.";
def SNAPSHOT_SEQ_MARKER: "__SNAPSHOT_SEQ__";

def text: if type == "string" then . else "" end;
def mapping: if type == "object" then . else {} end;
def among($choices): . as $value | $choices | any(.[]; . == $value);
def strip: sub("\\A\\s+"; "") | sub("\\s+\\z"; "");
def words: [splits("\\s+")] | map(select(length > 0));
def collapsed_spaces: words | join(" ");
def utc_timestamp: now | floor | strftime("%Y-%m-%dT%H:%M:%SZ");
def repeated($count): if $count > 0 then . * $count else "" end;
def violation($message): error({violation: $message});
def has_all($keys): . as $document | $keys | all(.[]; . as $key | $document | has($key));
def with_default($key; $value): if has($key) then .[$key] else $value end;

def is_review_agent: type == "string" and among(REVIEW_AGENT_TYPES);
def is_uuid: type == "string" and test(UUID_PATTERN);

def joined_response_text:
  if type == "array" then map(select(type == "object" and (.text | type) == "string") | .text) | join("\n") else "" end;

def any_running_subagent:
  type == "array" and any(.[]; mapping | .type == "subagent" and (.status | among(IN_FLIGHT_STATUSES)));

def hook_payload:
  ((try fromjson catch {}) | mapping) as $document
  | ($document.tool_input | mapping) as $input
  | ($document.tool_response | mapping) as $response
  | ($document.reason | text) as $reason
  | ($document.agent_id | text) as $agent_id
  | {
      session_id: ($document.session_id | text | strip | if is_uuid then . else "" end),
      cwd: ($document.cwd | text),
      tool_name: ($document.tool_name | text),
      tool_input: $input,
      subagent_type: ($input.subagent_type | text),
      spawn_prompt: ($input.prompt | text),
      response_text: ($response.content | joined_response_text),
      task_subject: ($input.subject | text),
      created_task_id: ($response.task | mapping | .id | text),
      updated_task_id: ($input.taskId | text),
      new_task_status: ($response.statusChange | mapping | .to | text),
      agent_type: ($document.agent_type | text),
      last_assistant_message: ($document.last_assistant_message | text),
      stop_hook_active: ($document.stop_hook_active == true),
      session_event: (if $reason != "" then $reason else ($document.source | text) end),
      session_transcript_path: ($document.transcript_path | text),
      background_work_running: ($document.background_tasks | any_running_subagent),
      agent_id: (if $agent_id != "" then $agent_id else ($response.agentId | text) end),
      transcript_path: ($document.agent_transcript_path | text),
      launched_in_background: ($response.status == "async_launched")
    };

def is_finished_review: (.subagent_type | is_review_agent) and .agent_id != "" and (.launched_in_background | not);

def is_fence_opening: test("\\A[ \\t]*```[\\w+-]*[ \\t]*\\r?\\z");
def is_fence_closing: test("\\A[ \\t]*```[ \\t]*\\z");

def fenced_blocks:
  reduce split("\n")[] as $line ({blocks: [], open: null};
    if .open == null then
      (if ($line | is_fence_opening) then .open = [] else . end)
    elif ($line | is_fence_closing) then
      .blocks += [.open | map(. + "\n") | join("")] | .open = null
    else
      .open += [$line]
    end)
  | .blocks;

def last_block_text:
  (if type == "string" then fenced_blocks else [] end)
  | map(select(index(SCHEMA) != null))
  | if length == 0 then violation("no fenced json block with \"schema\": \"\(SCHEMA)\" found") else last end;

def without_parse_echo: (index(" (while parsing")) as $cut | if $cut == null then . else .[0:$cut] end;

def decoded_block:
  (try fromjson catch violation("findings block is not valid JSON: " + (tostring | without_parse_echo)))
  | if type == "object" then . else violation("findings block must be a JSON object") end;

def within_length_limit($key; $where):
  FIELD_LENGTH_LIMITS[$key] as $limit
  | if $limit != null and length > $limit then violation("\($where)\($key) is longer than \($limit) characters") else . end;

def required_choice($key; $choices; $where):
  .[$key] as $value
  | if ($value | type) == "string" and ($value | among($choices)) then $value
    else violation("\($where)\($key) is missing or not one of \($choices | join(", "))") end;

def optional_choice($key; $choices; $where): if .[$key] == null then null else required_choice($key; $choices; $where) end;

def required_text($key; $where):
  .[$key] as $value
  | if ($value | type) != "string" or ($value | strip) == "" then violation("\($where)\($key) must be a non-empty string")
    else $value | within_length_limit($key; $where) end;

def optional_text($key; $where):
  .[$key] as $value
  | if $value != null and ($value | type) != "string" then violation("\($where)\($key) must be a string or null")
    elif $value == null or $value == "" then null
    else $value | within_length_limit($key; $where) end;

def optional_incident_id($where):
  .incident as $value
  | if $value == null then null
    elif ($value | type) == "string" and ($value | test("\\AF[0-9]{1,4}\\z")) then $value
    else violation("\($where)incident must be a prior incident id like F3, or null") end;

def pack_slug:
  if . == null then null
  else ascii_downcase | gsub("[^a-z0-9]+"; "-") | sub("\\A-"; "") | sub("-\\z"; "") | if . == "" then null else . end end;

def normalised_source:
  split(":") as $parts
  | if ($parts | length) == 1 then . else ($parts[1:] | join(":")) as $pack | $parts[0] + ":" + (($pack | pack_slug) // $pack) end;

def locator: [.type, .file, .symbol] | join("|");

def require_pack_attribution($where):
  if .type == "compliance" then (required_text("pack"; $where) | empty), (required_text("rule_title"; $where) | empty) else empty end;

def finding_from($where):
  if type != "object" then violation("\($where[0:-1]) must be a JSON object")
  else
    . as $entry
    | [$entry | require_pack_attribution($where)]
    | ($entry | required_choice("severity"; SEVERITIES; $where)) as $severity
    | ($entry | required_choice("type"; TYPES; $where)) as $type
    | ($entry | required_text("file"; $where)) as $file
    | ($entry | required_text("symbol"; $where)) as $symbol
    | ($entry | required_text("what"; $where)) as $what
    | ($entry | optional_text("pack"; $where) | pack_slug) as $pack
    | ($entry | optional_text("rule_title"; $where)) as $rule_title
    | ($entry | optional_text("rule_id"; $where)) as $rule_id
    | ($entry | optional_choice("outcome"; OUTCOMES; $where)) as $outcome
    | ($entry | optional_incident_id($where)) as $incident
    | {severity: $severity, type: $type, file: $file, symbol: $symbol, what: $what, pack: $pack,
       rule_title: $rule_title, outcome: $outcome, incident: $incident, rule_id: $rule_id}
    | if (locator | length) > MAX_FINGERPRINT_LENGTH then
        violation("\($where)file and \($where)symbol are too long together: their fingerprint exceeds \(MAX_FINGERPRINT_LENGTH) characters")
      else . end
  end;

def findings_from:
  if type != "array" then violation("findings must be a list (use [] for a clean pass)")
  elif length > MAX_FINDINGS then violation("findings holds more than \(MAX_FINDINGS) entries")
  else . as $entries | [range(0; length) as $index | $entries[$index] | finding_from("findings[\($index)].")] end;

def block_from:
  required_choice("schema"; [SCHEMA]; "") as $schema
  | .source as $source
  | (if ($source | type) == "string" and ($source | test("\\A(?:" + SOURCE_PATTERN + ")\\z")) then $source
     else violation("source is missing or does not match \(SOURCE_PATTERN)") end) as $valid_source
  | required_choice("mode"; MODES; "") as $mode
  | (.findings | findings_from) as $findings
  | {source: ($valid_source | normalised_source), mode: $mode, findings: $findings};

def parse_block:
  try {block: (last_block_text | decoded_block | block_from)}
  catch (if type == "object" and has("violation") then {failure: .violation} else error end);

def normalised_words: ascii_downcase | collapsed_spaces;

def what_it_is:
  if (.rule_id // "") != "" then "rule:" + .rule_id
  elif (.rule_title // "") != "" then "rule:" + (.rule_title | normalised_words)
  else .what | normalised_words end;

def fingerprint: [locator, what_it_is] | join("|");

def leading_slashes: if startswith("//") and (startswith("///") | not) then 2 elif startswith("/") then 1 else 0 end;

def posix_normpath:
  if . == "" then "."
  else
    leading_slashes as $initial
    | reduce (split("/")[] | select(. != "" and . != ".")) as $part ([];
        if $part != ".." or ($initial == 0 and length == 0) or (length > 0 and .[-1] == "..") then . + [$part]
        elif length > 0 then .[0:-1]
        else . end)
    | ("/" | repeated($initial)) + join("/")
    | if . == "" then "." else . end
  end;

def path_like:
  gsub("\\\\"; "/")
  | if . == "" then "."
    else leading_slashes as $initial
      | [split("/")[] | select(. != "" and . != ".")] | ("/" | repeated($initial)) + join("/")
      | if . == "" then "." else . end end;

def root_prefix: path_like | rtrimstr("/") + "/";

def normalised_file($roots):
  (strip | gsub("\\\\"; "/") | posix_normpath) as $path
  | (first($roots[] | root_prefix as $prefix | select($path | startswith($prefix)) | $path[($prefix | length):]) // $path)
  | sub("\\A\\.claude/worktrees/[^/]+/"; "");

def normalised_symbol: strip | if endswith("()") then .[0:-2] else . end;

def bounded_what: collapsed_spaces | .[0:MAX_WHAT_LENGTH];

def normalised_block($roots):
  .findings |= map(.file |= normalised_file($roots) | .symbol |= normalised_symbol | .what |= bounded_what);

def loaded_incident:
  if type == "object" and has_all(INCIDENT_REQUIRED_KEYS) then
    {
      id: .id, fingerprint: .fingerprint, severity: .severity, type: .type, what: .what, file: .file, symbol: .symbol,
      first_seen_at: .first_seen_at, status_changed_at: .status_changed_at,
      sources: with_default("sources"; []), rules: with_default("rules"; []), status: with_default("status"; "open"),
      task_id: with_default("task_id"; null), todo_subject: with_default("todo_subject"; ""),
      history: with_default("history"; []), cleared_by: with_default("cleared_by"; [])
    }
  else error("unreadable incident") end;

def loaded_ledger:
  fromjson
  | if type == "object" and has_all(LEDGER_REQUIRED_KEYS) and (.incidents | type) == "array" then
      {
        schema: .schema, run_id: .run_id, state: .state, started_at: .started_at, closed_at: .closed_at,
        incidents: (.incidents | map(loaded_incident)), unparsed: .unparsed, orphan_todos: .orphan_todos, totals: .totals,
        recorded_passes: with_default("recorded_passes"; {}), session_ids: with_default("session_ids"; [])
      }
    else error("unreadable ledger") end;

def readable_ledger: if . == "" then {ledger: null, log: []} else try {ledger: loaded_ledger, log: []} catch {ledger: null, log: ["load unreadable-ledger"]} end;

def load_run($ledger_text; $copy_text):
  ($ledger_text | readable_ledger) as $primary
  | if $primary.ledger != null then $primary
    else ($copy_text | readable_ledger) as $copy | {ledger: $copy.ledger, log: ($primary.log + $copy.log)} end;

def new_ledger($run_id; $now):
  {schema: SCHEMA, run_id: $run_id, state: "active", started_at: $now, closed_at: null,
   incidents: [], unparsed: [], orphan_todos: [], totals: {}, recorded_passes: {}, session_ids: []};

def start_run($run_id; $now): if . == null then new_ledger($run_id; $now) else .state = "active" | .closed_at = null end;

def set_status($status; $via; $now): .status = $status | .status_changed_at = $now | .history += [{status: $status, at: $now, via: $via}];

def count_by(key): reduce .[] as $item ({}; .[$item | key | tostring] += 1);

def packs_of: [.rules[]? | .pack | select(. != null and . != false and . != "")] | unique;

def compute_totals:
  . as $incidents
  | def counted($status): $incidents | map(select(.status == $status)) | length;
  {
    found: length,
    fixed: counted("fixed"),
    claimed_fixed: counted("claimed_fixed"),
    deferred: counted("deferred"),
    dropped: counted("dropped"),
    open: (counted("open") + counted("fixing")),
    by_severity: count_by(.severity),
    by_type: count_by(.type),
    by_pack: (reduce ($incidents[] | packs_of[]) as $pack ({}; .[$pack] += 1))
  };

def todo_subject:
  (.rules[0] // {}) as $rule
  | "\(FIX_PREFIX) #\(.id)] \(.severity | ascii_upcase) \(.type) \($rule.pack // "-") \(.file):\(.symbol) — \(.what)";

def todo_creation_instruction: "Create exactly these todos with TaskCreate, one per line, copying each subject verbatim:\n" + join("\n");

def todoless_instruction($ids):
  "Without the TaskCreate tool, resolve each one instead: fix it and re-run the source that raised it with mode=re-review, or defer it by writing its line below in your reply with the reason filled in:\n"
  + ($ids | map("[Anthara-Defer #\(.)] <reason>") | join("\n"))
  + "\nTo drop a false positive instead, write its drop line:\n"
  + ($ids | map("[Anthara-Drop #\(.)] <reason>") | join("\n"));

def incident_id_from_subject: [capture("\\A\\[Anthara-Fix #(?<id>F[0-9]+)\\]") | .id] | .[0];
def is_fix_subject: startswith(FIX_PREFIX);
def is_deferred_subject: sub("\\s+\\z"; "") | endswith(DEFERRED_SUFFIX);

def first_incident_index(condition): [.incidents | to_entries[] | select(.value | condition) | .key] | .[0];

def named_incident_index($finding):
  if $finding.incident == null then null
  else first_incident_index(.id == $finding.incident and .file == $finding.file) end;

def raised_by($source): .sources | any(.[]; . == $source);

def prior_incident_index($finding; $source):
  ($finding | fingerprint) as $key
  | named_incident_index($finding) // first_incident_index(raised_by($source) and .fingerprint == $key);

def next_incident_id: "F\(([.incidents[].id[1:] | tonumber] | max // 0) + 1)";

def new_incident($id; $finding; $now):
  {
    id: $id, fingerprint: ($finding | fingerprint), severity: $finding.severity, type: $finding.type, what: $finding.what,
    file: $finding.file, symbol: $finding.symbol, first_seen_at: $now, status_changed_at: $now,
    sources: [], rules: [], status: "open", task_id: null, todo_subject: "", history: [], cleared_by: []
  }
  | set_status("open"; "review"; $now);

def rules_of($verified):
  if (.pack // "") != "" and (.rule_title // "") != "" then
    [{pack: .pack, rule_title: .rule_title, rule_id: .rule_id, rule_verified: $verified}]
  else [] end;

def with_source($source): if raised_by($source) then . else .sources += [$source] end;

def absorb($finding; $source; $rules):
  .severity = ([.severity, $finding.severity] | min_by(. as $severity | SEVERITIES | index($severity)))
  | with_source($source)
  | reduce $rules[] as $rule (.; if any(.rules[]; . == $rule) then . else .rules += [$rule] end);

def merge_finding($finding; $source; $verified; $now):
  ($finding | fingerprint) as $key
  | (named_incident_index($finding) // first_incident_index(.fingerprint == $key)) as $existing
  | ($finding | rules_of($verified)) as $rules
  | if $existing != null then {ledger: (.incidents[$existing] |= absorb($finding; $source; $rules)), new_id: null}
    else next_incident_id as $id
      | .incidents += [new_incident($id; $finding; $now) | absorb($finding; $source; $rules)]
      | {ledger: ., new_id: $id} end;

def rule_is_in_prompt($prompt): (.rule_id // .rule_title // "") as $cited | $cited != "" and ($prompt | index($cited)) != null;

def merge_new_findings($findings; $source; $prompt; $now):
  reduce $findings[] as $finding ({ledger: ., new_ids: []};
    (.ledger | merge_finding($finding; $source; ($finding | rule_is_in_prompt($prompt)); $now)) as $merged
    | .ledger = $merged.ledger
    | .new_ids += (if $merged.new_id == null then [] else [$merged.new_id] end));

def reopened($source; $via; $now):
  .cleared_by -= [$source]
  | if .status | among(REOPENABLE_STATUSES) then set_status("open"; $via; $now) else . end;

def cleared_by_every_source: (.sources | length) > 0 and ((.sources - .cleared_by) | length) == 0;

def changed_ids: $changed | split("\n") | map(select(. != ""));

def is_unclaimed: .status == "open" and (.task_id == null or .task_id == "" or .task_id == false);

def counts_as_clear: (is_unclaimed | not) or (.id | among(changed_ids));

def confirmable: .status == "claimed_fixed" or is_unclaimed;

def cleared($source; $via; $now):
  if counts_as_clear | not then .
  else
    (if any(.cleared_by[]; . == $source) then . else .cleared_by += [$source] end)
    | if confirmable and cleared_by_every_source then set_status("fixed"; $via; $now) else . end
  end;

def record_first_pass($block; $prompt; $now): merge_new_findings($block.findings; $block.source; $prompt; $now);

def record_re_review($block; $prompt; $now):
  $block.source as $source
  | . as $before
  | [$block.findings[] | . as $finding | {finding: $finding, index: ($before | prior_incident_index($finding; $source))}] as $restated
  | ([$restated[] | .index | select(. != null)]) as $present
  | reduce $present[] as $index (.; .incidents[$index] |= with_source($source))
  | reduce range(0; .incidents | length) as $index (.;
      if .incidents[$index] | raised_by($source) | not then .
      elif $present | any(.[]; . == $index) then .incidents[$index] |= reopened($source; "re-review"; $now)
      else .incidents[$index] |= cleared($source; "re-review"; $now) end)
  | merge_new_findings([$restated[] | select(.index == null) | .finding]; $source; $prompt; $now);

def outcome_applied($outcome; $source; $now):
  if $outcome == "closed" then cleared($source; "re-verify"; $now)
  elif $outcome == "still-failing" then reopened($source; "re-verify"; $now)
  else . end;

def record_re_verify($block; $now):
  $block.source as $source
  | reduce $block.findings[] as $finding ({ledger: ., new_ids: []};
      (.ledger | prior_incident_index($finding; $source)) as $index
      | if $index != null then .ledger.incidents[$index] |= outcome_applied($finding.outcome; $source; $now)
        elif $finding.outcome | among(PASSING_OUTCOMES) then .
        else (.ledger | merge_finding($finding; $source; false; $now)) as $merged
          | .ledger = $merged.ledger
          | .new_ids += (if $merged.new_id == null then [] else [$merged.new_id] end) end);

def recorded_mode($block; $prompt; $now):
  if $block.mode == "first-pass" then record_first_pass($block; $prompt; $now)
  elif $block.mode == "re-review" then record_re_review($block; $prompt; $now)
  else record_re_verify($block; $now) end;

def with_subjects($new_ids):
  .incidents |= map(if .id | among($new_ids) then .todo_subject = todo_subject else . end);

def incidents_with_ids($ids): [.incidents[] | select(.id | among($ids))];

def checksum: reduce explode[] as $code (5381; (. * 33 + $code) % 4294967296);

def pass_signature: [.mode] + ([.findings[] | fingerprint + "|" + (.outcome // "")] | sort) | join("\n");

def pass_key($agent_id): if $agent_id == "" then "" else $agent_id + "#" + (pass_signature | checksum | tostring) end;

def record_pass($review; $roots; $now):
  ($review.block | normalised_block($roots)) as $block
  | ($block | pass_key($review.agent_id)) as $key
  | if .recorded_passes | has($key) then {ledger: ., new_ids: .recorded_passes[$key], changed: false}
    else
      recorded_mode($block; $review.prompt; $now) as $outcome
      | ($outcome.ledger | with_subjects($outcome.new_ids)
         | if $key != "" then .recorded_passes[$key] = $outcome.new_ids else . end) as $ledger
      | {ledger: $ledger, new_ids: $outcome.new_ids, changed: true}
    end;

def handoff_text($new_incidents; $ledger_path):
  "Anthara findings ledger: \($ledger_path) (\($new_incidents | length) new incidents recorded)."
  + if ($new_incidents | length) == 0 then " Create no [Anthara-Fix] todos for this review pass."
    else " " + ($new_incidents | map(.todo_subject) | todo_creation_instruction) + "\n" + todoless_instruction($new_incidents | map(.id)) end;

def handoff_context($new_incidents; $ledger_path):
  {hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: handoff_text($new_incidents; $ledger_path)}};

def assistant_text_of:
  if type != "object" then ""
  elif .type == "text" then .text | text
  elif .type == "tool_use" and .name == HANDBACK_TOOL then .input | mapping | .message | text
  else "" end;

def assistant_texts:
  (mapping | .message) as $message
  | if ($message | type) == "object" and $message.role == "assistant" and ($message.content | type) == "array"
    then $message.content[] | assistant_text_of | select(. != "")
    else empty end;

def agent_outputs:
  try ([split("\n")[] | select(test("\\S")) | fromjson] | [.[] | assistant_texts] | reverse) catch [];

def spawn_prompt_in: try (split("\n")[0] | fromjson | mapping | .message | mapping | .content | text) catch "";

def forked_from:
  [split("\n")[0:FORK_SCAN_LINES][]
   | (try fromjson catch null) | mapping | .forkedFrom | mapping | .sessionId | text | select(. != "")]
  | .[0] // "";

def latest_block($message; $transcript):
  ($message | parse_block) as $final
  | if $final.block != null then $final
    else [$transcript | agent_outputs[] | parse_block | select(.block != null)] | .[0] // $final end;

def block_reason($failure):
  "Your findings block is invalid: \($failure). Re-emit your complete hand-back, unchanged, ending with exactly one fenced json block that follows the \(SCHEMA) contract in your agent instructions.";

def unresolved: [.incidents[] | select(.status | among(NON_TERMINAL_STATUSES))];

def NOTE_OUTCOMES: {Defer: {status: "deferred", via: "defer-note"}, Drop: {status: "dropped", via: "drop-note"}};

def noted_outcomes:
  reduce (scan("\\[Anthara-(Defer|Drop) #(F[0-9]+)\\]")) as $note ({};
    if has($note[1]) then . else .[$note[1]] = NOTE_OUTCOMES[$note[0]] end);

def with_notes($notes; $now):
  .incidents |= map(
    $notes[.id] as $outcome
    | if $outcome != null and (.status | among(NON_TERMINAL_STATUSES)) then set_status($outcome.status; $outcome.via; $now) else . end);

def unclaimed_lines: [.incidents[] | select(is_unclaimed) | [.id, .first_seen_at, .file] | join("\t")] | join("\n");

def finalise($now):
  .state = (if (unresolved | length) > 0 then "incomplete" else "closed" end)
  | .closed_at = $now
  | .totals = (.incidents | compute_totals);

def totals_line($ledger_path):
  .totals as $totals
  | ([["found", "found"], ["fixed", "fixed"], ["claimed_fixed", "claimed"], ["deferred", "deferred"], ["dropped", "dropped"], ["open", "open"]]
     | map("\(.[1]) \($totals[.[0]] // 0)") | join(" · ")) as $counts
  | ([["high", "H"], ["medium", "M"], ["low", "L"]] | map("\(.[1]) \(($totals.by_severity // {})[.[0]] // 0)") | join(" · ")) as $severities
  | ([.unparsed[] | .source // "?"] | unique) as $unparsed_sources
  | "Anthara findings — \($counts)  (\($severities))  → \($ledger_path)"
    + if ($unparsed_sources | length) == 0 then "" else "  unparsed: " + ($unparsed_sources | join(", ")) end;

def stop_block_reason($pending):
  [$pending[] | select(.task_id != null and .task_id != "" and .task_id != false)] as $tracked
  | [$pending[] | select(.task_id == null or .task_id == "" or .task_id == false)] as $untracked
  | [
      (if ($tracked | length) == 0 then empty
       else "These incidents are not yet terminal: \($tracked | map("\(.id) (\(.status))") | join(", ")). Finish their [Anthara-Fix] todos before ending the turn." end),
      (if ($untracked | length) == 0 then empty
       else "These incidents have no [Anthara-Fix] todo yet. " + ($untracked | map(if .todo_subject != "" and .todo_subject != null then .todo_subject else todo_subject end) | todo_creation_instruction)
         + "\n" + todoless_instruction($untracked | map(.id)) end)
    ]
  | join("\n");

def briefed_prompt($spawn_prompt):
  .incidents[0:MAX_BRIEFED_INCIDENTS]
  | map("\(.id) [\(.sources | join(", "))] \(.severity) \(.type) \(.file):\(.symbol) — \(.what)")
  | $spawn_prompt + "\n\n" + ([BRIEF_HEADER] + . + [BRIEF_INSTRUCTION] | join("\n"));

def next_task_status($incident; $update):
  if $update.new_status == "completed" and ((if $update.new_subject != "" then $update.new_subject else $incident.todo_subject end) | is_deferred_subject) then "deferred"
  elif $update.new_status == "in_progress" then "fixing"
  elif $update.new_status == "completed" then "claimed_fixed"
  elif $update.new_status == "deleted" then "dropped"
  else null end;

def task_update_applied($update; $now):
  . as $incident
  | next_task_status($incident; $update) as $status
  | (if $update.new_subject != "" then .todo_subject = $update.new_subject else . end)
  | (if $status == "fixing" and .status != "fixing" then .cleared_by = [] else . end)
  | (if $status != null then set_status($status; "todo"; $now) else . end)
  | if .status == "claimed_fixed" and cleared_by_every_source then set_status("fixed"; "re-review"; $now) else . end;

def snapshot_payload($reporting_session):
  {
    anthara_session_id: .run_id,
    session_id: $reporting_session,
    snapshot_seq: SNAPSHOT_SEQ_MARKER,
    session_ids: .session_ids[0:20],
    run_state: .state,
    incidents: [.incidents[0:500][] | {
      id, severity, type: .type, what, file, symbol,
      sources: .sources[0:16], rules: .rules[0:16],
      status, first_seen_at, status_changed_at
    }],
    unparsed: [.unparsed[0:50][] | {source: .source[0:128], reason: .reason[0:500], at}]
  };

def snapshot_seq: (now * 1000000 | floor | tostring) + "000";

def assignment($name; $value): "\($name)=\($value | @sh)";

def emitted($reporting_session):
  (.ledger | if . == null then null else .totals = (.incidents | compute_totals) end) as $ledger
  | snapshot_seq as $seq
  | (("0" | repeated(20 - ($seq | length))) + $seq) as $padded_seq
  | [
      assignment("ENGINE_RESPONSE"; if .response == null then "" else .response | tojson end),
      assignment("ENGINE_LEDGER"; if $ledger == null then "" else $ledger | tojson end),
      assignment("ENGINE_POINT"; if .point == true then "1" else "" end),
      assignment("ENGINE_TASK_MAP"; if .task_map == null then "" else .task_map | tojson end),
      assignment("ENGINE_SNAPSHOT"; if $ledger == null then "" else $ledger | snapshot_payload($reporting_session) | tojson | sub("\"" + SNAPSHOT_SEQ_MARKER + "\""; $seq) end),
      assignment("ENGINE_SNAPSHOT_RUN"; if $ledger == null then "" else $ledger.run_id end),
      assignment("ENGINE_SNAPSHOT_SEQ"; $padded_seq),
      assignment("ENGINE_LOG"; (.log // []) | join("\n"))
    ]
  | .[];

def flag: if . then "1" else "" end;

def fields_output:
  $payload | hook_payload as $payload
  | [
      assignment("FIELD_SESSION_ID"; $payload.session_id),
      assignment("FIELD_CWD"; $payload.cwd),
      assignment("FIELD_TOOL_NAME"; $payload.tool_name),
      assignment("FIELD_IS_REVIEW_SUBAGENT"; $payload.subagent_type | is_review_agent | flag),
      assignment("FIELD_IS_FINISHED_REVIEW"; $payload | is_finished_review | flag),
      assignment("FIELD_IS_REVIEW_AGENT"; $payload.agent_type | is_review_agent | flag),
      assignment("FIELD_AGENT_TRANSCRIPT"; $payload.transcript_path),
      assignment("FIELD_SESSION_TRANSCRIPT"; $payload.session_transcript_path),
      assignment("FIELD_SESSION_EVENT"; $payload.session_event),
      assignment("FIELD_BACKGROUND_RUNNING"; $payload.background_work_running | flag)
    ]
  | .[];

def recorded_review_result($review; $loaded; $now):
  ($loaded.ledger | start_run($session_id; $now) | record_pass($review; [$project, $repo_root]; $now)) as $pass
  | {
      ledger: (if $pass.changed then $pass.ledger else null end),
      point: $pass.changed,
      new_incidents: ($pass.ledger | incidents_with_ids($pass.new_ids)),
      log: $loaded.log
    };

def brief_result:
  $payload | hook_payload as $payload
  | load_run($ledger; $copy) as $loaded
  | if ($payload.subagent_type | is_review_agent | not) or $loaded.ledger == null or $loaded.ledger.state != "active" or ($loaded.ledger.incidents | length) == 0 then {log: $loaded.log}
    else {
      response: {hookSpecificOutput: {hookEventName: "PreToolUse", updatedInput: ($payload.tool_input + {prompt: ($loaded.ledger | briefed_prompt($payload.spawn_prompt))})}},
      log: $loaded.log
    } end;

def review_stop_result:
  $payload | hook_payload as $payload
  | utc_timestamp as $now
  | latest_block($payload.last_assistant_message; $transcript) as $outcome
  | if ($payload.agent_type | is_review_agent | not) then {}
    elif $outcome.block != null then
      load_run($ledger; $copy) as $loaded
      | recorded_review_result({block: $outcome.block, agent_id: $payload.agent_id, prompt: ($transcript | spawn_prompt_in)}; $loaded; $now)
      | del(.new_incidents)
    elif $payload.stop_hook_active | not then {response: {decision: "block", reason: block_reason($outcome.failure)}}
    else
      load_run($ledger; $copy) as $loaded
      | {
          ledger: ($loaded.ledger | start_run($session_id; $now) | .unparsed += [{source: $payload.agent_type, reason: $outcome.failure, at: $now}]),
          point: true,
          log: $loaded.log
        }
    end;

def hand_back_result:
  $payload | hook_payload as $payload
  | ($payload.response_text | parse_block) as $outcome
  | if ($payload | is_finished_review | not) or $outcome.block == null then {}
    else
      load_run($ledger; $copy) as $loaded
      | recorded_review_result({block: $outcome.block, agent_id: $payload.agent_id, prompt: $payload.spawn_prompt}; $loaded; utc_timestamp)
      | .response = handoff_context(.new_incidents; $ledger_path)
      | del(.new_incidents)
    end;

def task_map_document: if . == "" then {} else fromjson end;

def created_task_result($payload; $loaded; $task_map; $now):
  ($payload.task_subject | incident_id_from_subject) as $incident_id
  | $loaded.ledger
  | (if $incident_id == null then null else first_incident_index(.id == $incident_id) end) as $index
  | if $index == null then
      {ledger: (.orphan_todos += [{subject_id: $incident_id, task_id: $payload.created_task_id, at: $now}]), task_map: $task_map}
    else
      .incidents[$index].id as $id
      | {
          ledger: (.incidents[$index] |= (.task_id = $payload.created_task_id | .todo_subject = $payload.task_subject)),
          task_map: ($task_map + {($payload.created_task_id): $id})
        }
    end;

def updated_task_result($payload; $loaded; $task_map; $now):
  {task_id: $payload.updated_task_id, new_status: $payload.new_task_status, new_subject: $payload.task_subject} as $update
  | $loaded.ledger
  | ($task_map[$update.task_id] // null) as $incident_id
  | (if $incident_id == null then null else first_incident_index(.id == $incident_id) end) as $index
  | if $index == null or (.incidents[$index].status | among(TERMINAL_STATUSES)) then {ledger: ., task_map: $task_map}
    else
      (if .state != "active" then start_run(.run_id; $now) else . end)
      | {ledger: (.incidents[$index] |= task_update_applied($update; $now)), task_map: $task_map}
    end;

def track_result:
  $payload | hook_payload as $payload
  | utc_timestamp as $now
  | if $payload.tool_name == "TaskCreate" and (($payload.task_subject | is_fix_subject | not) or $payload.created_task_id == "") then {}
    elif ($payload.tool_name | among(["TaskCreate", "TaskUpdate"]) | not) then {}
    else
      load_run($ledger; $copy) as $loaded
      | if $loaded.ledger == null then {log: $loaded.log}
        else ($task_map | task_map_document) as $map
          | (if $payload.tool_name == "TaskCreate" then created_task_result($payload; $loaded; $map; $now)
             else updated_task_result($payload; $loaded; $map; $now) end)
          | .log = $loaded.log
        end
    end;

def active_run($loaded): if $loaded.ledger != null and $loaded.ledger.state == "active" then $loaded.ledger else null end;

def closing_result($noted; $payload; $log):
  ($noted | unresolved) as $pending
  | if ($pending | length) > 0 and ($payload.stop_hook_active | not) then
      {ledger: $noted, response: {decision: "block", reason: stop_block_reason($pending)}, log: $log}
    else
      ($noted | finalise(utc_timestamp)) as $closed
      | {ledger: $closed, response: {systemMessage: ($closed | totals_line($ledger_path))}, log: $log}
    end;

def close_result:
  $payload | hook_payload as $payload
  | load_run($ledger; $copy) as $loaded
  | active_run($loaded) as $run
  | if $run == null then {log: $loaded.log}
    else
      ($run | with_notes($payload.last_assistant_message | noted_outcomes; utc_timestamp)) as $noted
      | if $payload.background_work_running then {ledger: (if $noted == $run then null else $noted end), log: $loaded.log}
        else closing_result($noted; $payload; $loaded.log) end
    end;

def unclaimed_files_output:
  load_run($ledger; $copy) as $loaded
  | assignment("ENGINE_UNCLAIMED"; if $loaded.ledger == null then "" else $loaded.ledger | unclaimed_lines end);

def end_result:
  load_run($ledger; $copy) as $loaded
  | active_run($loaded) as $run
  | if $run == null then {log: $loaded.log} else {ledger: ($run | finalise(utc_timestamp)), log: $loaded.log} end;

def fork_parent_output:
  ($transcript | forked_from) as $parent
  | assignment("ENGINE_PARENT_ID"; if ($parent | is_uuid) and $parent != $session_id then $parent else "" end);

def adopt_result:
  load_run($ledger; $copy) as $loaded
  | if $loaded.ledger == null then {log: $loaded.log}
    else {
      ledger: ($loaded.ledger | if any(.session_ids[]; . == $session_id) then . else .session_ids += [$session_id] end),
      task_map: ($task_map | task_map_document),
      point: true,
      log: $loaded.log
    } end;

if $op == "fields" then fields_output
elif $op == "fork-parent" then fork_parent_output
elif $op == "unclaimed-files" then unclaimed_files_output
elif $op == "brief" then brief_result | emitted($session_id)
elif $op == "review-stop" then review_stop_result | emitted($session_id)
elif $op == "hand-back" then hand_back_result | emitted($session_id)
elif $op == "track" then track_result | emitted($session_id)
elif $op == "close" then close_result | emitted($session_id)
elif $op == "end" then end_result | emitted($session_id)
elif $op == "adopt" then adopt_result | emitted($session_id)
elif $op == "sweep" then end_result | emitted($session_id)
else error("unknown operation \($op)") end
