import { navigateTo } from "./router.svelte.js";

export interface CommandDef {
  id: string;
  name: string;
  description: string;
  args: string;
  action: (arg?: string) => void;
}

export const builtinCommands: CommandDef[] = [
  {
    id: "search",
    name: "/search",
    description: "Searches entities, shows results as chat cards inline",
    args: "<query>",
    action: () => navigateTo("search"),
  },
  {
    id: "graph",
    name: "/graph",
    description: "Navigates to graph view centered on entity",
    args: "<entity>",
    action: () => navigateTo("graph"),
  },
  {
    id: "import",
    name: "/import",
    description: "Navigates to import view",
    args: "[url]",
    action: () => navigateTo("import"),
  },
  {
    id: "help",
    name: "/help",
    description: "Shows command list or specific command help",
    args: "[command]",
    action: () => {},
  },
  {
    id: "clear",
    name: "/clear",
    description: "Clears current conversation with confirmation",
    args: "",
    action: () => {},
  },
  {
    id: "export",
    name: "/export",
    description: "Exports conversation in specified format",
    args: "<format>",
    action: () => {},
  },
];

export function matchCommands(prefix: string): CommandDef[] {
  if (!prefix) return builtinCommands;
  return builtinCommands.filter((c) => c.name.startsWith(prefix));
}
