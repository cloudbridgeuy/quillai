import { useEffect, useState } from "react";
import { defaultLocale, formatMessage } from "../../i18n/messages";
import { ManuscriptApiError } from "../../manuscripts/client";
import { timesOfDay, type ImageShot, type LoreEntry, type LoreImage, type TimeOfDay } from "../../manuscripts/lore_client";
import type { useLore } from "../../manuscripts/use_lore";
import { createSettingsClient } from "../../settings/client";
import { Alert } from "../../ui/Alert";
import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { calculateImageCost, snapshotImagePricing } from "../cost";
import { formatInspectorUsd } from "../inspector_cost";
import { effectivePoint, resolveLoreEntry } from "../lore_states";

const purposes = ["portrait", "map", "render"] as const;
type Purpose = (typeof purposes)[number];

function shotFor(entry: LoreEntry, purpose: Purpose, timeOfDay: TimeOfDay): ImageShot | undefined {
  if (entry.kind !== "place") return undefined;
  return purpose === "render" ? { purpose, timeOfDay } : { purpose };
}

export function LoreImages({ lore, entry, disabled }: { lore: ReturnType<typeof useLore>; entry: LoreEntry; disabled: boolean }) {
  const [images, setImages] = useState<LoreImage[]>([]);
  const [cost, setCost] = useState<string | null>(null);
  const [blocked, setBlocked] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [purpose, setPurpose] = useState<Purpose>("portrait");
  const [timeOfDay, setTimeOfDay] = useState<TimeOfDay>("day");
  const universeId = lore.universeId;
  const pointsById = new Map(lore.points.map(point => [point.id, point]));
  const point = effectivePoint(lore.piece, pointsById);
  const leftOut = resolveLoreEntry(entry, entry.states ?? [], point, pointsById).level === "leave_out";
  const shot = shotFor(entry, purpose, timeOfDay);
  useEffect(() => {
    if (universeId === null) return;
    let active = true;
    lore.client.listImages({ universeId, loreId: entry.id }).then(value => { if (active) setImages(value); }, () => {});
    const settings = createSettingsClient();
    Promise.all([settings.getModelCatalog(), settings.getAiSettings().catch(() => null)]).then(([catalog, saved]) => {
      const modelId = saved?.image_model ?? catalog?.defaults.image;
      const result = calculateImageCost(snapshotImagePricing(catalog, modelId === undefined ? { kind: "server-default-unresolved" } : { kind: "known", modelId }), 1);
      if (active) setCost(result.kind === "known" ? formatInspectorUsd(result.usd) : formatMessage(defaultLocale, "lore.images.unknownCost"));
    });
    return () => { active = false; };
  }, [universeId, entry.id, lore.client]);
  async function generate() {
    if (universeId === null) return;
    setPending(true);
    setError("");
    try {
      const image = await lore.client.createImage({ universeId, loreId: entry.id }, point?.id ?? null, shot);
      setImages(value => [...value, image]);
    } catch (failure) {
      const status = failure instanceof ManuscriptApiError ? failure.status : 0;
      if (status === 422) setBlocked(true);
      setError(status === 503 ? formatMessage(defaultLocale, "lore.images.chooseModel")
        : failure instanceof Error ? failure.message : formatMessage(defaultLocale, "lore.images.failed"));
    } finally { setPending(false); }
  }
  if (universeId === null) return null;
  return <div className="space-y-2">
    {images.length > 0 ? <div className="flex flex-wrap gap-2">{images.map(image => <img key={image.id} className="h-16 w-16 object-cover"
      src={lore.client.imageUrl({ universeId, loreId: entry.id }, image.id)} alt={formatMessage(defaultLocale, "lore.images.alt", { name: entry.name })} />)}</div> : null}
    {cost === null ? null : <p className="editor-note">{formatMessage(defaultLocale, "lore.images.cost", { cost })}</p>}
    {blocked || leftOut || shot === undefined ? null : <Select label={formatMessage(defaultLocale, "lore.images.purpose")} value={purpose} disabled={disabled || pending}
      options={purposes.map(value => ({ value, label: formatMessage(defaultLocale, `lore.images.${value}`) }))}
      onChange={value => { const next = purposes.find(option => option === value); if (next) setPurpose(next); }} />}
    {blocked || leftOut || shot?.purpose !== "render" ? null : <Select label={formatMessage(defaultLocale, "lore.images.timeOfDay")} value={timeOfDay} disabled={disabled || pending}
      options={timesOfDay.map(value => ({ value, label: formatMessage(defaultLocale, `lore.images.${value}`) }))}
      onChange={value => { const next = timesOfDay.find(option => option === value); if (next) setTimeOfDay(next); }} />}
    {blocked || leftOut ? null : <Button size="sm" disabled={disabled || pending} onClick={() => void generate()}>{formatMessage(defaultLocale, "lore.images.generate")}</Button>}
    {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
  </div>;
}
