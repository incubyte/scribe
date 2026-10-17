#!/usr/bin/env bash
# PreToolUse hook for Write tool
# Blocks writes to the local .claude/projects/ memory directory.
# Forces Claude to use Fabric memory (mcp__plugin_anthara_anthara-fabric__add_private_memory / mcp__plugin_anthara_anthara-fabric__add_shared_memory) instead.

set -euo pipefail

INPUT=$(cat)

FILE_PATH=$(echo "$INPUT" | node -e "
  let d=''; process.stdin.on('data',c=>d+=c);
  process.stdin.on('end',()=>{
    try{
      const obj=JSON.parse(d);
      const p=(obj.tool_input.file_path||'').toLowerCase().split(String.fromCharCode(92)).join('/');
      console.log(p);
    }catch{console.log('')}
  });
" 2>/dev/null)

if echo "$FILE_PATH" | grep -q '\.claude/projects.*memory'; then
  node -e "
    console.log(JSON.stringify({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'deny',
        permissionDecisionReason: 'BLOCKED: Do not write to local file memory (.claude/projects/*/memory/). Use Fabric instead: mcp__plugin_anthara_anthara-fabric__add_private_memory for personal preferences, mcp__plugin_anthara_anthara-fabric__add_shared_memory for team conventions.'
      }
    }));
  "
  exit 0
fi

# Not a memory path — allow the write
exit 0
