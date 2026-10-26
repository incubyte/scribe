import { blockName } from "./agendaTimes";
import type { AgendaBlock } from "../types";

interface AgendaBlockItemProps {
  readonly block: AgendaBlock;
  readonly ended: boolean;
}

export function AgendaBlockItem({ block, ended }: AgendaBlockItemProps) {
  const style = {
    top: `${block.top_minutes}px`,
    height: `${block.height_minutes}px`,
    left: `${(block.column / block.columns) * 100}%`,
    width: `${100 / block.columns}%`,
  };
  const className = ended ? "agenda__block agenda__block--ended" : "agenda__block";

  return (
    <li className={className} style={style} tabIndex={0} aria-label={blockName(block, ended)}>
      <span className="agenda__subject" aria-hidden="true">
        {block.subject}
      </span>
      {block.is_recurring ? (
        <span className="agenda__recurring" aria-hidden="true">
          ↻
        </span>
      ) : null}
      {block.is_teams ? (
        <span className="agenda__tag" aria-hidden="true">
          Teams
        </span>
      ) : null}
    </li>
  );
}
