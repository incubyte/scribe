#!/usr/bin/env bash
# PreToolUse hook for mcp__plugin_anthara_anthara-fabric__add_shared_memory
# Forces user permission dialog before ANY shared memory save.
# Returns permissionDecision: "ask" so the user always sees a confirmation prompt.

set -euo pipefail

INPUT=$(cat)

DATA=$(echo "$INPUT" | node -e "
  let d=''; process.stdin.on('data',c=>d+=c);
  process.stdin.on('end',()=>{
    try{
      const obj=JSON.parse(d);
      console.log(obj.tool_input.data||'(empty)');
    }catch{console.log('(could not parse)')}
  });
" 2>/dev/null)

node -e "
  console.log(JSON.stringify({
    hookSpecificOutput: {
      hookEventName: 'PreToolUse',
      permissionDecision: 'ask',
      permissionDecisionReason: 'Shared org memory save requires your approval. Content: ' + $(node -e "console.log(JSON.stringify(process.argv[1]))" -- "$DATA")
    }
  }));
"
exit 0
