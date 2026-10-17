"use strict";

const fs = require("fs");

const SCHEMA = "anthara.findings/v1";
const REVIEW_AGENT_NAMES = ["reviewer", "pack-reviewer", "browser-verifier"];
const REVIEW_AGENT_TYPES = [...REVIEW_AGENT_NAMES, ...REVIEW_AGENT_NAMES.map((name) => `anthara:${name}`)];
const SEVERITIES = ["high", "medium", "low"];
const TYPES = ["architecture", "integration", "code-quality", "test-quality", "ai-ergonomics", "accessibility", "compliance", "browser"];
const MODES = ["first-pass", "re-review", "re-verify"];
const OUTCOMES = ["closed", "still-failing", "unverified"];
const PASSING_OUTCOMES = ["closed", "unverified"];
const IN_FLIGHT_STATUSES = ["pending", "running", "paused"];
const NON_TERMINAL_STATUSES = ["open", "fixing"];
const TERMINAL_STATUSES = ["fixed", "deferred", "dropped"];
const REOPENABLE_STATUSES = ["claimed_fixed", "fixed"];
const FIELD_LENGTH_LIMITS = { file: 1024, symbol: 512, pack: 128, rule_title: 256, rule_id: 64 };
const SOURCE_PATTERN = "reviewer|browser-verifier|pack-reviewer:[^\\n]{1,128}";
const SOURCE_REGEX = new RegExp(`^(?:${SOURCE_PATTERN})$`, "u");
const UUID_REGEX = /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
const INCIDENT_ID_REGEX = /^F[0-9]{1,4}$/;
const FIX_SUBJECT_REGEX = /^\[Anthara-Fix #(F[0-9]+)\]/;
const FENCE_OPENING = /^[ \t]*```[\w+-]*[ \t]*\r?$/u;
const FENCE_CLOSING = /^[ \t]*```[ \t]*$/;
const WORKTREE_PREFIX = /^\.claude\/worktrees\/[^/]+\//;
const FIX_PREFIX = "[Anthara-Fix";
const DEFERRED_SUFFIX = "(deferred)";
const MAX_FINDINGS = 500;
const MAX_FINGERPRINT_LENGTH = 1024;
const MAX_WHAT_LENGTH = 200;
const MAX_BRIEFED_INCIDENTS = 100;
const HANDBACK_TOOL = "SubagentHandback";
const FORK_SCAN_LINES = 200;
const INCIDENT_REQUIRED_KEYS = ["id", "fingerprint", "severity", "type", "what", "file", "symbol", "first_seen_at", "status_changed_at"];
const LEDGER_REQUIRED_KEYS = ["schema", "run_id", "state", "started_at", "closed_at", "incidents", "unparsed", "orphan_todos", "totals"];
const BRIEF_HEADER = "Anthara findings already recorded in this run (id [sources] severity type file:symbol — what):";
const BRIEF_INSTRUCTION =
  'In your findings block, set a finding\'s "incident" to the id above that it restates, even if you now word its file or symbol differently; set "incident" to null for a new finding. In mode=re-review, every incident from your own source that you do not report is read as cleared.';
const SNAPSHOT_SEQ_MARKER = "__SNAPSHOT_SEQ__";

class ContractViolation extends Error {}

const isAbsent = (value) => value === undefined || value === null;
const isObject = (value) => value !== null && typeof value === "object" && !Array.isArray(value);
const text = (value) => (typeof value === "string" ? value : "");
const mapping = (value) => (isObject(value) ? value : {});
const among = (value, choices) => choices.includes(value);
const characters = (value) => Array.from(value);
const lengthOf = (value) => characters(value).length;
const sliced = (value, start, end) => characters(value).slice(start, end).join("");
const strip = (value) => value.replace(/^\s+/u, "").replace(/\s+$/u, "");
const collapsedSpaces = (value) => value.split(/\s+/u).filter((word) => word.length > 0).join(" ");
const asciiLower = (value) => value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());
const utcTimestamp = () => `${new Date().toISOString().slice(0, 19)}Z`;
const hasAll = (document, keys) => keys.every((key) => Object.prototype.hasOwnProperty.call(document, key));
const withDefault = (document, key, value) => (Object.prototype.hasOwnProperty.call(document, key) ? document[key] : value);
const sameValue = (left, right) => JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
const isReviewAgent = (agentType) => typeof agentType === "string" && among(agentType, REVIEW_AGENT_TYPES);
const isUuid = (value) => typeof value === "string" && UUID_REGEX.test(value);
const violation = (message) => {
  throw new ContractViolation(message);
};

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (!isObject(value)) return value;
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])]));
}

function parsedOr(raw, fallback) {
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

function joinedResponseText(content) {
  if (!Array.isArray(content)) return "";
  return content.filter((part) => isObject(part) && typeof part.text === "string").map((part) => part.text).join("\n");
}

function anyRunningSubagent(tasks) {
  return Array.isArray(tasks) && tasks.some((task) => mapping(task).type === "subagent" && among(mapping(task).status, IN_FLIGHT_STATUSES));
}

function sessionFields(document) {
  const sessionId = strip(text(document.session_id));
  const reason = text(document.reason);
  return {
    session_id: isUuid(sessionId) ? sessionId : "",
    cwd: text(document.cwd),
    session_event: reason !== "" ? reason : text(document.source),
    session_transcript_path: text(document.transcript_path),
    background_work_running: anyRunningSubagent(document.background_tasks),
  };
}

function toolFields(document, input, response) {
  return {
    tool_name: text(document.tool_name),
    tool_input: input,
    subagent_type: text(input.subagent_type),
    spawn_prompt: text(input.prompt),
    response_text: joinedResponseText(response.content),
    task_subject: text(input.subject),
    created_task_id: text(mapping(response.task).id),
    updated_task_id: text(input.taskId),
    new_task_status: text(mapping(response.statusChange).to),
  };
}

function agentFields(document, response) {
  const agentId = text(document.agent_id);
  return {
    agent_type: text(document.agent_type),
    last_assistant_message: text(document.last_assistant_message),
    stop_hook_active: document.stop_hook_active === true,
    agent_id: agentId !== "" ? agentId : text(response.agentId),
    transcript_path: text(document.agent_transcript_path),
    launched_in_background: response.status === "async_launched",
  };
}

function hookPayload(raw) {
  const document = mapping(parsedOr(raw, {}));
  const input = mapping(document.tool_input);
  const response = mapping(document.tool_response);
  return { ...sessionFields(document), ...toolFields(document, input, response), ...agentFields(document, response) };
}

const isFinishedReview = (payload) => isReviewAgent(payload.subagent_type) && payload.agent_id !== "" && !payload.launched_in_background;

function fencedBlocks(message) {
  const blocks = [];
  let open = null;
  for (const line of message.split("\n")) {
    if (open === null) {
      if (FENCE_OPENING.test(line)) open = [];
    } else if (FENCE_CLOSING.test(line)) {
      blocks.push(open.map((content) => `${content}\n`).join(""));
      open = null;
    } else {
      open.push(line);
    }
  }
  return blocks;
}

function lastBlockText(message) {
  const candidates = (typeof message === "string" ? fencedBlocks(message) : []).filter((block) => block.includes(SCHEMA));
  if (candidates.length === 0) violation(`no fenced json block with "schema": "${SCHEMA}" found`);
  return candidates[candidates.length - 1];
}

function decodedBlock(blockText) {
  let document;
  try {
    document = JSON.parse(blockText);
  } catch (error) {
    violation(`findings block is not valid JSON: ${error.message}`);
  }
  if (!isObject(document)) violation("findings block must be a JSON object");
  return document;
}

function withinLengthLimit(value, key, where) {
  const limit = FIELD_LENGTH_LIMITS[key];
  if (limit !== undefined && lengthOf(value) > limit) violation(`${where}${key} is longer than ${limit} characters`);
  return value;
}

function requiredChoice(document, key, choices, where) {
  const value = document[key];
  if (typeof value !== "string" || !among(value, choices)) violation(`${where}${key} is missing or not one of ${choices.join(", ")}`);
  return value;
}

function optionalChoice(document, key, choices, where) {
  return isAbsent(document[key]) ? null : requiredChoice(document, key, choices, where);
}

function requiredText(document, key, where) {
  const value = document[key];
  if (typeof value !== "string" || strip(value) === "") violation(`${where}${key} must be a non-empty string`);
  return withinLengthLimit(value, key, where);
}

function optionalText(document, key, where) {
  const value = document[key];
  if (isAbsent(value)) return null;
  if (typeof value !== "string") violation(`${where}${key} must be a string or null`);
  return value === "" ? null : withinLengthLimit(value, key, where);
}

function optionalIncidentId(entry, where) {
  const value = entry.incident;
  if (isAbsent(value)) return null;
  if (typeof value === "string" && INCIDENT_ID_REGEX.test(value)) return value;
  return violation(`${where}incident must be a prior incident id like F3, or null`);
}

function packSlug(name) {
  if (name === null) return null;
  const slug = asciiLower(name).replace(/[^a-z0-9]+/g, "-").replace(/^-/, "").replace(/-$/, "");
  return slug === "" ? null : slug;
}

function normalisedSource(source) {
  const parts = source.split(":");
  if (parts.length === 1) return source;
  const pack = parts.slice(1).join(":");
  return `${parts[0]}:${packSlug(pack) ?? pack}`;
}

const locator = (finding) => [finding.type, finding.file, finding.symbol].join("|");

function requirePackAttribution(entry, where) {
  if (entry.type !== "compliance") return;
  requiredText(entry, "pack", where);
  requiredText(entry, "rule_title", where);
}

function locatedFinding(entry, where) {
  return {
    severity: requiredChoice(entry, "severity", SEVERITIES, where),
    type: requiredChoice(entry, "type", TYPES, where),
    file: requiredText(entry, "file", where),
    symbol: requiredText(entry, "symbol", where),
    what: requiredText(entry, "what", where),
  };
}

function attributionOf(entry, where) {
  return {
    pack: packSlug(optionalText(entry, "pack", where)),
    rule_title: optionalText(entry, "rule_title", where),
    rule_id: optionalText(entry, "rule_id", where),
    outcome: optionalChoice(entry, "outcome", OUTCOMES, where),
    incident: optionalIncidentId(entry, where),
  };
}

function withBoundedFingerprint(finding, where) {
  if (lengthOf(locator(finding)) > MAX_FINGERPRINT_LENGTH) {
    violation(`${where}file and ${where}symbol are too long together: their fingerprint exceeds ${MAX_FINGERPRINT_LENGTH} characters`);
  }
  return finding;
}

function findingFrom(entry, where) {
  if (!isObject(entry)) violation(`${where.slice(0, -1)} must be a JSON object`);
  requirePackAttribution(entry, where);
  return withBoundedFingerprint({ ...locatedFinding(entry, where), ...attributionOf(entry, where) }, where);
}

function findingsFrom(entries) {
  if (!Array.isArray(entries)) violation("findings must be a list (use [] for a clean pass)");
  if (entries.length > MAX_FINDINGS) violation(`findings holds more than ${MAX_FINDINGS} entries`);
  return entries.map((entry, index) => findingFrom(entry, `findings[${index}].`));
}

function blockFrom(document) {
  requiredChoice(document, "schema", [SCHEMA], "");
  const source = document.source;
  if (typeof source !== "string" || !SOURCE_REGEX.test(source)) violation(`source is missing or does not match ${SOURCE_PATTERN}`);
  const mode = requiredChoice(document, "mode", MODES, "");
  const findings = findingsFrom(document.findings);
  return { source: normalisedSource(source), mode, findings };
}

function parseBlock(message) {
  try {
    return { block: blockFrom(decodedBlock(lastBlockText(message))) };
  } catch (error) {
    if (error instanceof ContractViolation) return { failure: error.message };
    throw error;
  }
}

const normalisedWords = (value) => collapsedSpaces(asciiLower(value));

function whatItIs(finding) {
  if (finding.rule_id) return `rule:${finding.rule_id}`;
  if (finding.rule_title) return `rule:${normalisedWords(finding.rule_title)}`;
  return normalisedWords(finding.what);
}

const fingerprint = (finding) => [locator(finding), whatItIs(finding)].join("|");

function leadingSlashes(path) {
  if (path.startsWith("//") && !path.startsWith("///")) return 2;
  return path.startsWith("/") ? 1 : 0;
}

const isCurrentOrEmpty = (part) => part === "" || part === ".";
const endsInParent = (parts) => parts.length > 0 && parts[parts.length - 1] === "..";
const climbsAboveStart = (initial, parts) => initial === 0 && parts.length === 0;

function keepsPart(part, initial, parts) {
  return part !== ".." || climbsAboveStart(initial, parts) || endsInParent(parts);
}

function resolvedParts(path, initial) {
  const parts = [];
  for (const part of path.split("/").filter((candidate) => !isCurrentOrEmpty(candidate))) {
    if (keepsPart(part, initial, parts)) parts.push(part);
    else parts.pop();
  }
  return parts;
}

function posixNormpath(path) {
  if (path === "") return ".";
  const initial = leadingSlashes(path);
  return "/".repeat(initial) + resolvedParts(path, initial).join("/") || ".";
}

function pathLike(path) {
  const forward = path.replace(/\\/g, "/");
  if (forward === "") return ".";
  const parts = forward.split("/").filter((part) => part !== "" && part !== ".");
  return "/".repeat(leadingSlashes(forward)) + parts.join("/") || ".";
}

const rootPrefix = (root) => `${pathLike(root).replace(/\/$/, "")}/`;

function normalisedFile(file, roots) {
  const path = posixNormpath(strip(file).replace(/\\/g, "/"));
  const prefix = roots.map(rootPrefix).find((candidate) => path.startsWith(candidate));
  const relative = prefix === undefined ? path : path.slice(prefix.length);
  return relative.replace(WORKTREE_PREFIX, "");
}

function normalisedSymbol(symbol) {
  const trimmed = strip(symbol);
  return trimmed.endsWith("()") ? trimmed.slice(0, -2) : trimmed;
}

const boundedWhat = (what) => sliced(collapsedSpaces(what), 0, MAX_WHAT_LENGTH);

function normalisedBlock(block, roots) {
  return {
    ...block,
    findings: block.findings.map((finding) => ({
      ...finding,
      file: normalisedFile(finding.file, roots),
      symbol: normalisedSymbol(finding.symbol),
      what: boundedWhat(finding.what),
    })),
  };
}

function trackingFields(document) {
  return {
    sources: withDefault(document, "sources", []),
    rules: withDefault(document, "rules", []),
    status: withDefault(document, "status", "open"),
    task_id: withDefault(document, "task_id", null),
    todo_subject: withDefault(document, "todo_subject", ""),
    history: withDefault(document, "history", []),
    cleared_by: withDefault(document, "cleared_by", []),
  };
}

function loadedIncident(document) {
  if (!isObject(document) || !hasAll(document, INCIDENT_REQUIRED_KEYS)) throw new Error("unreadable incident");
  const required = Object.fromEntries(INCIDENT_REQUIRED_KEYS.map((key) => [key, document[key]]));
  return { ...required, ...trackingFields(document) };
}

function loadedLedger(raw) {
  const document = JSON.parse(raw);
  if (!isObject(document) || !hasAll(document, LEDGER_REQUIRED_KEYS) || !Array.isArray(document.incidents)) {
    throw new Error("unreadable ledger");
  }
  return {
    schema: document.schema,
    run_id: document.run_id,
    state: document.state,
    started_at: document.started_at,
    closed_at: document.closed_at,
    incidents: document.incidents.map(loadedIncident),
    unparsed: document.unparsed,
    orphan_todos: document.orphan_todos,
    totals: document.totals,
    recorded_passes: withDefault(document, "recorded_passes", {}),
    session_ids: withDefault(document, "session_ids", []),
  };
}

function readableLedger(raw) {
  if (raw === "") return { ledger: null, log: [] };
  try {
    return { ledger: loadedLedger(raw), log: [] };
  } catch {
    return { ledger: null, log: ["load unreadable-ledger"] };
  }
}

function loadRun(ledgerText, copyText) {
  const primary = readableLedger(ledgerText);
  if (primary.ledger !== null) return primary;
  const copy = readableLedger(copyText);
  return { ledger: copy.ledger, log: [...primary.log, ...copy.log] };
}

function newLedger(runId, now) {
  return {
    schema: SCHEMA,
    run_id: runId,
    state: "active",
    started_at: now,
    closed_at: null,
    incidents: [],
    unparsed: [],
    orphan_todos: [],
    totals: {},
    recorded_passes: {},
    session_ids: [],
  };
}

function startRun(ledger, runId, now) {
  if (ledger === null) return newLedger(runId, now);
  ledger.state = "active";
  ledger.closed_at = null;
  return ledger;
}

function setStatus(incident, status, via, now) {
  incident.status = status;
  incident.status_changed_at = now;
  incident.history.push({ status, at: now, via });
}

function countBy(items, key) {
  const counts = {};
  for (const item of items) {
    const name = String(key(item));
    counts[name] = (counts[name] || 0) + 1;
  }
  return counts;
}

function packsOf(incident) {
  const packs = (Array.isArray(incident.rules) ? incident.rules : []).map((rule) => mapping(rule).pack).filter((pack) => pack);
  return [...new Set(packs)].sort();
}

function computeTotals(incidents) {
  const counted = (status) => incidents.filter((incident) => incident.status === status).length;
  const byPack = {};
  for (const pack of incidents.flatMap(packsOf)) byPack[pack] = (byPack[pack] || 0) + 1;
  return {
    found: incidents.length,
    fixed: counted("fixed"),
    claimed_fixed: counted("claimed_fixed"),
    deferred: counted("deferred"),
    dropped: counted("dropped"),
    open: counted("open") + counted("fixing"),
    by_severity: countBy(incidents, (incident) => incident.severity),
    by_type: countBy(incidents, (incident) => incident.type),
    by_pack: byPack,
  };
}

function todoSubject(incident) {
  const rule = incident.rules[0] || {};
  return `${FIX_PREFIX} #${incident.id}] ${incident.severity.toUpperCase()} ${incident.type} ${rule.pack || "-"} ${incident.file}:${incident.symbol} — ${incident.what}`;
}

const todoCreationInstruction = (subjects) =>
  `Create exactly these todos with TaskCreate, one per line, copying each subject verbatim:\n${subjects.join("\n")}`;

function incidentIdFromSubject(subject) {
  const match = FIX_SUBJECT_REGEX.exec(subject);
  return match ? match[1] : null;
}

const isFixSubject = (subject) => subject.startsWith(FIX_PREFIX);
const isDeferredSubject = (subject) => subject.replace(/\s+$/u, "").endsWith(DEFERRED_SUFFIX);

function namedIncident(ledger, finding) {
  if (finding.incident === null || finding.incident === undefined) return undefined;
  return ledger.incidents.find((incident) => incident.id === finding.incident && incident.file === finding.file);
}

const raisedBy = (incident, source) => incident.sources.includes(source);

function priorIncident(ledger, finding, source) {
  const key = fingerprint(finding);
  return namedIncident(ledger, finding) || ledger.incidents.find((incident) => raisedBy(incident, source) && incident.fingerprint === key);
}

function nextIncidentId(ledger) {
  const numbers = ledger.incidents.map((incident) => {
    const number = Number(incident.id.slice(1));
    if (!Number.isFinite(number)) throw new Error("unreadable incident id");
    return number;
  });
  return `F${Math.max(0, ...numbers) + 1}`;
}

function untrackedFields() {
  return { sources: [], rules: [], status: "open", task_id: null, todo_subject: "", history: [], cleared_by: [] };
}

function newIncident(id, finding, now) {
  const { severity, type, what, file, symbol } = finding;
  const located = { id, fingerprint: fingerprint(finding), severity, type, what, file, symbol };
  const incident = { ...located, first_seen_at: now, status_changed_at: now, ...untrackedFields() };
  setStatus(incident, "open", "review", now);
  return incident;
}

function rulesOf(finding, verified) {
  if (!finding.pack || !finding.rule_title) return [];
  return [{ pack: finding.pack, rule_title: finding.rule_title, rule_id: finding.rule_id, rule_verified: verified }];
}

function withSource(incident, source) {
  if (!raisedBy(incident, source)) incident.sources.push(source);
}

function absorb(incident, finding, source, rules) {
  incident.severity = [incident.severity, finding.severity].reduce((left, right) =>
    SEVERITIES.indexOf(right) < SEVERITIES.indexOf(left) ? right : left
  );
  withSource(incident, source);
  for (const rule of rules) {
    if (!incident.rules.some((existing) => sameValue(existing, rule))) incident.rules.push(rule);
  }
}

function mergeFinding(ledger, finding, source, verified, now) {
  const key = fingerprint(finding);
  const existing = namedIncident(ledger, finding) || ledger.incidents.find((incident) => incident.fingerprint === key);
  const rules = rulesOf(finding, verified);
  if (existing) {
    absorb(existing, finding, source, rules);
    return null;
  }
  const incident = newIncident(nextIncidentId(ledger), finding, now);
  absorb(incident, finding, source, rules);
  ledger.incidents.push(incident);
  return incident.id;
}

function ruleIsInPrompt(finding, prompt) {
  const cited = finding.rule_id || finding.rule_title || "";
  return cited !== "" && prompt.includes(cited);
}

function mergeNewFindings(ledger, findings, source, prompt, now) {
  return findings.map((finding) => mergeFinding(ledger, finding, source, ruleIsInPrompt(finding, prompt), now)).filter((id) => id !== null);
}

function reopened(incident, source, via, now) {
  incident.cleared_by = incident.cleared_by.filter((cleared) => cleared !== source);
  if (among(incident.status, REOPENABLE_STATUSES)) setStatus(incident, "open", via, now);
}

const clearedByEverySource = (incident) =>
  incident.sources.length > 0 && incident.sources.every((source) => incident.cleared_by.includes(source));

const isUnclaimed = (incident) => incident.status === "open" && !incident.task_id;

const countsAsClear = (incident, changed) => !isUnclaimed(incident) || changed.has(incident.id);

const confirmable = (incident) => incident.status === "claimed_fixed" || isUnclaimed(incident);

function cleared(incident, source, via, now, changed) {
  if (!countsAsClear(incident, changed)) return;
  if (!incident.cleared_by.includes(source)) incident.cleared_by.push(source);
  if (confirmable(incident) && clearedByEverySource(incident)) setStatus(incident, "fixed", via, now);
}

function recordFirstPass(ledger, block, review, now) {
  return mergeNewFindings(ledger, block.findings, block.source, review.prompt, now);
}

function recordReReview(ledger, block, review, now) {
  const source = block.source;
  const restated = block.findings.map((finding) => ({ finding, incident: priorIncident(ledger, finding, source) }));
  const present = new Set(restated.filter((entry) => entry.incident).map((entry) => entry.incident));
  for (const incident of present) withSource(incident, source);
  for (const incident of ledger.incidents) {
    if (!raisedBy(incident, source)) continue;
    if (present.has(incident)) reopened(incident, source, "re-review", now);
    else cleared(incident, source, "re-review", now, review.changed);
  }
  const unmatched = restated.filter((entry) => !entry.incident).map((entry) => entry.finding);
  return mergeNewFindings(ledger, unmatched, source, review.prompt, now);
}

const OUTCOME_ACTIONS = {
  closed: (incident, source, now, changed) => cleared(incident, source, "re-verify", now, changed),
  "still-failing": (incident, source, now) => reopened(incident, source, "re-verify", now),
};

function reVerified(ledger, finding, source, now, changed) {
  const incident = priorIncident(ledger, finding, source);
  if (!incident) return among(finding.outcome, PASSING_OUTCOMES) ? null : mergeFinding(ledger, finding, source, false, now);
  const action = OUTCOME_ACTIONS[finding.outcome];
  if (action) action(incident, source, now, changed);
  return null;
}

function recordReVerify(ledger, block, review, now) {
  return block.findings.map((finding) => reVerified(ledger, finding, block.source, now, review.changed)).filter((id) => id !== null);
}

const MODE_RECORDERS = {
  "first-pass": recordFirstPass,
  "re-review": recordReReview,
  "re-verify": recordReVerify,
};

function checksum(text) {
  let sum = 5381;
  for (const character of text) sum = (sum * 33 + character.codePointAt(0)) % 4294967296;
  return sum;
}

function passSignature(block) {
  const findings = block.findings.map((finding) => `${fingerprint(finding)}|${finding.outcome ?? ""}`).sort();
  return [block.mode, ...findings].join("\n");
}

function passKey(block, agentId) {
  return agentId === "" ? "" : `${agentId}#${checksum(passSignature(block))}`;
}

function recordPass(ledger, review, roots, now) {
  const block = normalisedBlock(review.block, roots);
  const key = passKey(block, review.agent_id);
  if (Object.prototype.hasOwnProperty.call(ledger.recorded_passes, key)) {
    return { newIds: ledger.recorded_passes[key], changed: false };
  }
  const newIds = MODE_RECORDERS[block.mode](ledger, block, review, now);
  for (const incident of ledger.incidents) {
    if (newIds.includes(incident.id)) incident.todo_subject = todoSubject(incident);
  }
  if (key !== "") ledger.recorded_passes[key] = newIds;
  return { newIds, changed: true };
}

function todolessInstruction(ids) {
  return [
    "Without the TaskCreate tool, resolve each one instead: fix it and re-run the source that raised it with mode=re-review, or defer it by writing its line below in your reply with the reason filled in:",
    ...ids.map((id) => `[Anthara-Defer #${id}] <reason>`),
    "To drop a false positive instead, write its drop line:",
    ...ids.map((id) => `[Anthara-Drop #${id}] <reason>`),
  ].join("\n");
}

function handoffText(newIncidents, ledgerPath) {
  const header = `Anthara findings ledger: ${ledgerPath} (${newIncidents.length} new incidents recorded).`;
  if (newIncidents.length === 0) return `${header} Create no [Anthara-Fix] todos for this review pass.`;
  const subjects = newIncidents.map((incident) => incident.todo_subject);
  return `${header} ${todoCreationInstruction(subjects)}\n${todolessInstruction(newIncidents.map((incident) => incident.id))}`;
}

function assistantTextOf(item) {
  if (!isObject(item)) return "";
  if (item.type === "text") return text(item.text);
  if (item.type === "tool_use" && item.name === HANDBACK_TOOL) return text(mapping(item.input).message);
  return "";
}

function assistantTexts(entry) {
  const message = mapping(entry).message;
  if (!isObject(message) || message.role !== "assistant" || !Array.isArray(message.content)) return [];
  return message.content.map(assistantTextOf).filter((value) => value !== "");
}

function agentOutputs(transcript) {
  try {
    const entries = transcript.split("\n").filter((line) => /\S/u.test(line)).map((line) => JSON.parse(line));
    return entries.flatMap(assistantTexts).reverse();
  } catch {
    return [];
  }
}

function spawnPromptIn(transcript) {
  try {
    return text(mapping(mapping(JSON.parse(transcript.split("\n")[0])).message).content);
  } catch {
    return "";
  }
}

function forkedFrom(transcript) {
  for (const line of transcript.split("\n").slice(0, FORK_SCAN_LINES)) {
    const parent = text(mapping(mapping(parsedOr(line, null)).forkedFrom).sessionId);
    if (parent !== "") return parent;
  }
  return "";
}

function latestBlock(message, transcript) {
  const final = parseBlock(message);
  if (final.block) return final;
  for (const output of agentOutputs(transcript)) {
    const outcome = parseBlock(output);
    if (outcome.block) return outcome;
  }
  return final;
}

const blockReason = (failure) =>
  `Your findings block is invalid: ${failure}. Re-emit your complete hand-back, unchanged, ending with exactly one fenced json block that follows the ${SCHEMA} contract in your agent instructions.`;

const unresolved = (ledger) => ledger.incidents.filter((incident) => among(incident.status, NON_TERMINAL_STATUSES));

function finalise(ledger, now) {
  ledger.state = unresolved(ledger).length > 0 ? "incomplete" : "closed";
  ledger.closed_at = now;
  ledger.totals = computeTotals(ledger.incidents);
  return ledger;
}

function totalsLine(ledger, ledgerPath) {
  const totals = ledger.totals;
  const buckets = [["found", "found"], ["fixed", "fixed"], ["claimed_fixed", "claimed"], ["deferred", "deferred"], ["dropped", "dropped"], ["open", "open"]];
  const counts = buckets.map(([key, label]) => `${label} ${totals[key] ?? 0}`).join(" · ");
  const bySeverity = totals.by_severity || {};
  const severities = [["high", "H"], ["medium", "M"], ["low", "L"]].map(([key, label]) => `${label} ${bySeverity[key] ?? 0}`).join(" · ");
  const unparsedSources = [...new Set(ledger.unparsed.map((entry) => mapping(entry).source ?? "?"))].sort();
  const suffix = unparsedSources.length === 0 ? "" : `  unparsed: ${unparsedSources.join(", ")}`;
  return `Anthara findings — ${counts}  (${severities})  → ${ledgerPath}${suffix}`;
}

function stopBlockReason(pending) {
  const tracked = pending.filter((incident) => incident.task_id);
  const untracked = pending.filter((incident) => !incident.task_id);
  const reasons = [];
  if (tracked.length > 0) {
    const names = tracked.map((incident) => `${incident.id} (${incident.status})`).join(", ");
    reasons.push(`These incidents are not yet terminal: ${names}. Finish their [Anthara-Fix] todos before ending the turn.`);
  }
  if (untracked.length > 0) {
    const subjects = untracked.map((incident) => incident.todo_subject || todoSubject(incident));
    const ids = untracked.map((incident) => incident.id);
    reasons.push(`These incidents have no [Anthara-Fix] todo yet. ${todoCreationInstruction(subjects)}\n${todolessInstruction(ids)}`);
  }
  return reasons.join("\n");
}

function briefedPrompt(ledger, spawnPrompt) {
  const lines = ledger.incidents
    .slice(0, MAX_BRIEFED_INCIDENTS)
    .map((incident) => `${incident.id} [${incident.sources.join(", ")}] ${incident.severity} ${incident.type} ${incident.file}:${incident.symbol} — ${incident.what}`);
  return `${spawnPrompt}\n\n${[BRIEF_HEADER, ...lines, BRIEF_INSTRUCTION].join("\n")}`;
}

const TASK_TRANSITIONS = { in_progress: "fixing", completed: "claimed_fixed", deleted: "dropped" };

function nextTaskStatus(incident, update) {
  const subject = update.new_subject !== "" ? update.new_subject : incident.todo_subject;
  if (update.new_status === "completed" && isDeferredSubject(subject)) return "deferred";
  return TASK_TRANSITIONS[update.new_status] ?? null;
}

function startedFixDiscardsClears(incident, status) {
  if (status === "fixing" && incident.status !== "fixing") incident.cleared_by = [];
}

function confirmedWhenCleared(incident, now) {
  if (incident.status === "claimed_fixed" && clearedByEverySource(incident)) setStatus(incident, "fixed", "re-review", now);
}

function applyTaskUpdate(incident, update, now) {
  const status = nextTaskStatus(incident, update);
  if (update.new_subject !== "") incident.todo_subject = update.new_subject;
  startedFixDiscardsClears(incident, status);
  if (status !== null) setStatus(incident, status, "todo", now);
  confirmedWhenCleared(incident, now);
}

function incidentPayload(incident) {
  const { id, severity, type, what, file, symbol, status, first_seen_at, status_changed_at } = incident;
  const attribution = { sources: incident.sources.slice(0, 16), rules: incident.rules.slice(0, 16) };
  return { id, severity, type, what, file, symbol, ...attribution, status, first_seen_at, status_changed_at };
}

function unparsedPayload(entry) {
  return { source: sliced(entry.source, 0, 128), reason: sliced(entry.reason, 0, 500), at: entry.at };
}

function snapshotPayload(ledger, reportingSession) {
  return {
    anthara_session_id: ledger.run_id,
    session_id: reportingSession,
    snapshot_seq: SNAPSHOT_SEQ_MARKER,
    session_ids: ledger.session_ids.slice(0, 20),
    run_state: ledger.state,
    incidents: ledger.incidents.slice(0, 500).map(incidentPayload),
    unparsed: ledger.unparsed.slice(0, 50).map(unparsedPayload),
  };
}

const snapshotSeq = () => `${Math.floor((performance.timeOrigin + performance.now()) * 1000)}000`;
const shellQuoted = (value) => `'${value.replace(/'/g, "'\\''")}'`;
const assignment = (name, value) => `${name}=${shellQuoted(value)}`;
const flag = (value) => (value ? "1" : "");

const jsonOrEmpty = (value) => (value ? JSON.stringify(value) : "");
const withTotals = (ledger) => (ledger ? { ...ledger, totals: computeTotals(ledger.incidents) } : null);

function snapshotText(ledger, reportingSession, seq) {
  if (!ledger) return "";
  return JSON.stringify(snapshotPayload(ledger, reportingSession)).replace(`"${SNAPSHOT_SEQ_MARKER}"`, seq);
}

function emitted(result, reportingSession) {
  const ledger = withTotals(result.ledger);
  const seq = snapshotSeq();
  return [
    assignment("ENGINE_RESPONSE", jsonOrEmpty(result.response)),
    assignment("ENGINE_LEDGER", jsonOrEmpty(ledger)),
    assignment("ENGINE_POINT", flag(result.point === true)),
    assignment("ENGINE_TASK_MAP", jsonOrEmpty(result.task_map)),
    assignment("ENGINE_SNAPSHOT", snapshotText(ledger, reportingSession, seq)),
    assignment("ENGINE_SNAPSHOT_RUN", ledger ? ledger.run_id : ""),
    assignment("ENGINE_SNAPSHOT_SEQ", seq.padStart(20, "0")),
    assignment("ENGINE_LOG", (result.log || []).join("\n")),
  ];
}

function fieldsOutput(context) {
  const payload = hookPayload(context.payload);
  return [
    assignment("FIELD_SESSION_ID", payload.session_id),
    assignment("FIELD_CWD", payload.cwd),
    assignment("FIELD_TOOL_NAME", payload.tool_name),
    assignment("FIELD_IS_REVIEW_SUBAGENT", flag(isReviewAgent(payload.subagent_type))),
    assignment("FIELD_IS_FINISHED_REVIEW", flag(isFinishedReview(payload))),
    assignment("FIELD_IS_REVIEW_AGENT", flag(isReviewAgent(payload.agent_type))),
    assignment("FIELD_AGENT_TRANSCRIPT", payload.transcript_path),
    assignment("FIELD_SESSION_TRANSCRIPT", payload.session_transcript_path),
    assignment("FIELD_SESSION_EVENT", payload.session_event),
    assignment("FIELD_BACKGROUND_RUNNING", flag(payload.background_work_running)),
  ];
}

function recordedReview(context, review, loaded, now) {
  const ledger = startRun(loaded.ledger, context.sessionId, now);
  const pass = recordPass(ledger, { ...review, changed: context.changed }, [context.project, context.repoRoot], now);
  return {
    ledger: pass.changed ? ledger : null,
    point: pass.changed,
    newIncidents: ledger.incidents.filter((incident) => pass.newIds.includes(incident.id)),
    log: loaded.log,
  };
}

function briefResult(context) {
  const payload = hookPayload(context.payload);
  const loaded = loadRun(context.ledger, context.copy);
  const ledger = loaded.ledger;
  if (!isReviewAgent(payload.subagent_type) || ledger === null || ledger.state !== "active" || ledger.incidents.length === 0) {
    return { log: loaded.log };
  }
  const updatedInput = { ...payload.tool_input, prompt: briefedPrompt(ledger, payload.spawn_prompt) };
  return { response: { hookSpecificOutput: { hookEventName: "PreToolUse", updatedInput } }, log: loaded.log };
}

function reviewStopResult(context) {
  const payload = hookPayload(context.payload);
  if (!isReviewAgent(payload.agent_type)) return {};
  const now = utcTimestamp();
  const outcome = latestBlock(payload.last_assistant_message, context.transcript);
  if (outcome.block) {
    const review = { block: outcome.block, agent_id: payload.agent_id, prompt: spawnPromptIn(context.transcript) };
    const { ledger, point, log } = recordedReview(context, review, loadRun(context.ledger, context.copy), now);
    return { ledger, point, log };
  }
  if (!payload.stop_hook_active) return { response: { decision: "block", reason: blockReason(outcome.failure) } };
  const loaded = loadRun(context.ledger, context.copy);
  const ledger = startRun(loaded.ledger, context.sessionId, now);
  ledger.unparsed.push({ source: payload.agent_type, reason: outcome.failure, at: now });
  return { ledger, point: true, log: loaded.log };
}

function handBackResult(context) {
  const payload = hookPayload(context.payload);
  const outcome = parseBlock(payload.response_text);
  if (!isFinishedReview(payload) || !outcome.block) return {};
  const review = { block: outcome.block, agent_id: payload.agent_id, prompt: payload.spawn_prompt };
  const { newIncidents, ...result } = recordedReview(context, review, loadRun(context.ledger, context.copy), utcTimestamp());
  const additionalContext = handoffText(newIncidents, context.ledgerPath);
  return { ...result, response: { hookSpecificOutput: { hookEventName: "PostToolUse", additionalContext } } };
}

const taskMapDocument = (raw) => (raw === "" ? {} : JSON.parse(raw));

function createdTaskResult(payload, ledger, taskMap, now) {
  const incidentId = incidentIdFromSubject(payload.task_subject);
  const incident = incidentId === null ? undefined : ledger.incidents.find((candidate) => candidate.id === incidentId);
  if (!incident) {
    ledger.orphan_todos.push({ subject_id: incidentId, task_id: payload.created_task_id, at: now });
    return { ledger, task_map: taskMap };
  }
  incident.task_id = payload.created_task_id;
  incident.todo_subject = payload.task_subject;
  return { ledger, task_map: { ...taskMap, [payload.created_task_id]: incident.id } };
}

function mappedIncident(ledger, taskMap, taskId) {
  if (!Object.prototype.hasOwnProperty.call(taskMap, taskId)) return undefined;
  return ledger.incidents.find((candidate) => candidate.id === taskMap[taskId]);
}

function updatedTaskResult(payload, ledger, taskMap, now) {
  const update = { task_id: payload.updated_task_id, new_status: payload.new_task_status, new_subject: payload.task_subject };
  const incident = mappedIncident(ledger, taskMap, update.task_id);
  if (!incident || among(incident.status, TERMINAL_STATUSES)) return { ledger, task_map: taskMap };
  if (ledger.state !== "active") startRun(ledger, ledger.run_id, now);
  applyTaskUpdate(incident, update, now);
  return { ledger, task_map: taskMap };
}

const TASK_EVENTS = { TaskCreate: createdTaskResult, TaskUpdate: updatedTaskResult };
const isUnlinkableCreate = (payload) =>
  payload.tool_name === "TaskCreate" && (!isFixSubject(payload.task_subject) || payload.created_task_id === "");

function trackResult(context) {
  const payload = hookPayload(context.payload);
  const taskEvent = TASK_EVENTS[payload.tool_name];
  if (!taskEvent || isUnlinkableCreate(payload)) return {};
  const loaded = loadRun(context.ledger, context.copy);
  if (loaded.ledger === null) return { log: loaded.log };
  const result = taskEvent(payload, loaded.ledger, taskMapDocument(context.taskMap), utcTimestamp());
  return { ...result, log: loaded.log };
}

const activeRun = (loaded) => (loaded.ledger !== null && loaded.ledger.state === "active" ? loaded.ledger : null);

const NOTE_LINE = /\[Anthara-(Defer|Drop) #(F[0-9]+)\]/g;

const NOTE_OUTCOMES = {
  Defer: { status: "deferred", via: "defer-note" },
  Drop: { status: "dropped", via: "drop-note" },
};

function notedOutcomes(message) {
  const outcomes = new Map();
  for (const [, kind, id] of message.matchAll(NOTE_LINE)) {
    if (!outcomes.has(id)) outcomes.set(id, NOTE_OUTCOMES[kind]);
  }
  return outcomes;
}

function applyNotes(run, message, now) {
  const outcomes = notedOutcomes(message);
  const noted = run.incidents.filter((incident) => outcomes.has(incident.id) && among(incident.status, NON_TERMINAL_STATUSES));
  for (const incident of noted) {
    const outcome = outcomes.get(incident.id);
    setStatus(incident, outcome.status, outcome.via, now);
  }
  return noted.length > 0;
}

function closingResult(run, payload, context, log) {
  const pending = unresolved(run);
  if (pending.length > 0 && !payload.stop_hook_active) {
    return { ledger: run, response: { decision: "block", reason: stopBlockReason(pending) }, log };
  }
  const closed = finalise(run, utcTimestamp());
  return { ledger: closed, response: { systemMessage: totalsLine(closed, context.ledgerPath) }, log };
}

function closeResult(context) {
  const payload = hookPayload(context.payload);
  const loaded = loadRun(context.ledger, context.copy);
  const run = activeRun(loaded);
  if (run === null) return { log: loaded.log };
  const noted = applyNotes(run, payload.last_assistant_message, utcTimestamp());
  if (payload.background_work_running) return noted ? { ledger: run, log: loaded.log } : { log: loaded.log };
  return closingResult(run, payload, context, loaded.log);
}

function unclaimedFilesOutput(context) {
  const loaded = loadRun(context.ledger, context.copy);
  const incidents = loaded.ledger === null ? [] : loaded.ledger.incidents.filter(isUnclaimed);
  const lines = incidents.map((incident) => [incident.id, incident.first_seen_at, incident.file].join("\t"));
  return [assignment("ENGINE_UNCLAIMED", lines.join("\n"))];
}

function endResult(context) {
  const loaded = loadRun(context.ledger, context.copy);
  const run = activeRun(loaded);
  return run === null ? { log: loaded.log } : { ledger: finalise(run, utcTimestamp()), log: loaded.log };
}

function forkParentOutput(context) {
  const parent = forkedFrom(context.transcript);
  return [assignment("ENGINE_PARENT_ID", isUuid(parent) && parent !== context.sessionId ? parent : "")];
}

function adoptResult(context) {
  const loaded = loadRun(context.ledger, context.copy);
  if (loaded.ledger === null) return { log: loaded.log };
  const ledger = loaded.ledger;
  if (!ledger.session_ids.includes(context.sessionId)) ledger.session_ids.push(context.sessionId);
  return { ledger, task_map: taskMapDocument(context.taskMap), point: true, log: loaded.log };
}

const LEDGER_OPERATIONS = {
  brief: briefResult,
  "review-stop": reviewStopResult,
  "hand-back": handBackResult,
  track: trackResult,
  close: closeResult,
  end: endResult,
  adopt: adoptResult,
  sweep: endResult,
};

function readInput(path) {
  try {
    return fs.readFileSync(path, "utf8");
  } catch {
    return "";
  }
}

function contextFrom(argv) {
  const [sessionId, project, repoRoot, ledgerPath, payload, ledger, copy, taskMap, transcript, changed] = argv;
  return {
    sessionId,
    project,
    repoRoot,
    ledgerPath,
    payload: readInput(payload),
    ledger: readInput(ledger),
    copy: readInput(copy),
    taskMap: readInput(taskMap),
    transcript: readInput(transcript),
    changed: new Set(readInput(changed).split("\n").filter((id) => id !== "")),
  };
}

function run(operation, context) {
  if (operation === "fields") return fieldsOutput(context);
  if (operation === "fork-parent") return forkParentOutput(context);
  if (operation === "unclaimed-files") return unclaimedFilesOutput(context);
  const ledgerOperation = LEDGER_OPERATIONS[operation];
  if (!ledgerOperation) throw new Error(`unknown operation ${operation}`);
  return emitted(ledgerOperation(context), context.sessionId);
}

const [operation, ...rest] = process.argv.slice(2);
process.stdout.write(`${run(operation, contextFrom(rest)).join("\n")}\n`);
