import type { AgendaBlock, AgendaChip, AgendaView } from "../types";

export type AgendaItem =
  | { readonly kind: "block"; readonly block: AgendaBlock }
  | { readonly kind: "chip"; readonly chip: AgendaChip };

const topOf = (item: AgendaItem): number =>
  item.kind === "block" ? item.block.top_minutes : item.chip.top_minutes;

export function agendaItems(view: AgendaView): readonly AgendaItem[] {
  const blocks = view.blocks.map((block): AgendaItem => ({ kind: "block", block }));
  const chips = view.overflow.map((chip): AgendaItem => ({ kind: "chip", chip }));
  return [...blocks, ...chips].sort((first, second) => topOf(first) - topOf(second));
}

export function itemKey(item: AgendaItem): string {
  return item.kind === "block"
    ? item.block.event_id
    : `more-${item.chip.hidden.map((meeting) => meeting.event_id).join("-")}`;
}
