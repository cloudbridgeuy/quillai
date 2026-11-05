import type { CSSProperties } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import tokensCss from "../app/src/ui/tokens.css?raw";

type Token = { name: string; value: string };

const tokens: Token[] = [...tokensCss.matchAll(/(--[\w-]+):\s*([^;]+);/g)]
  .map(([, name, value]) => ({ name: name!, value: value!.trim() }))
  .filter((token) => token.value !== "initial");

const byName = new Map(tokens.map((token) => [token.name, token.value]));
const resolve = (value: string): string => value.replace(/var\((--[\w-]+)\)/g, (match, name: string) => byName.get(name) ?? match);

function previewStyle({ name, value }: Token): CSSProperties | null {
  const resolved = resolve(value);
  if (name.startsWith("--shadow-") || name.endsWith("-shadow")) return { boxShadow: resolved };
  if (name.startsWith("--radius-")) return { borderRadius: resolved, border: "1px solid var(--ink)" };
  if (name.startsWith("--color-") || CSS.supports("color", resolved)) return { background: resolved, border: "1px solid var(--rule)" };
  return null;
}

function TokenRow(token: Token) {
  const style = previewStyle(token);
  const textSize = token.name.startsWith("--text-") && !token.name.includes("--line-height");
  return (
    <tr>
      <td><code>{token.name}</code></td>
      <td><code>{token.value}</code></td>
      <td>{style ? <div style={{ width: 48, height: 32, ...style }} /> : textSize ? <span style={{ fontSize: token.value }}>Aa</span> : null}</td>
    </tr>
  );
}

function TokensTable() {
  return (
    <table style={{ borderSpacing: "16px 8px" }}>
      <thead><tr><th>Token</th><th>Value</th><th>Preview</th></tr></thead>
      <tbody>{tokens.map((token) => <TokenRow key={token.name} {...token} />)}</tbody>
    </table>
  );
}

const meta = { title: "UI/Tokens", component: TokensTable } satisfies Meta<typeof TokensTable>;

export default meta;

export const All: StoryObj<typeof meta> = {};
