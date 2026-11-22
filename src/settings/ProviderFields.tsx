import { useId, useState } from "react";
import { api, errorMessage } from "../api";
import type { ProviderId } from "../bindings/ProviderId";
import type { ProviderPreset } from "../bindings/ProviderPreset";
import type { Role } from "../bindings/Role";
import { ApiKeyField } from "./ApiKeyField";
import { Field } from "./Layout";

export type Endpoint = { provider: ProviderId; baseUrl: string; model: string };

type Check =
  | { state: "idle" }
  | { state: "checking" }
  | { state: "ok"; count: number }
  | { state: "error"; message: string };

const LOCAL_PROVIDERS: ProviderId[] = ["ollama", "lmstudio", "custom"];

function supportsRole(preset: ProviderPreset, providerRole: Role): boolean {
  return providerRole === "transcription" ? preset.transcribes : preset.cleansUp;
}

export function ProviderFields({
  providerRole,
  providers,
  value,
  onChange,
}: {
  providerRole: Role;
  providers: ProviderPreset[];
  value: Endpoint;
  onChange: (endpoint: Endpoint) => void;
}) {
  const [models, setModels] = useState<string[]>([]);
  const [check, setCheck] = useState<Check>({ state: "idle" });
  const modelListId = useId();
  const preset = providers.find((candidate) => candidate.id === value.provider);

  const choose = (id: ProviderId) => {
    const next = providers.find((candidate) => candidate.id === id);
    if (!next) return;
    const model = providerRole === "transcription" ? next.transcriptionModel : next.cleanupModel;
    onChange({ provider: id, baseUrl: next.baseUrl, model: model ?? "" });
    setModels([]);
    setCheck({ state: "idle" });
  };

  const checkConnection = async () => {
    setCheck({ state: "checking" });
    try {
      const available = await api.listModels(value.provider, providerRole, value.baseUrl);
      setModels(available);
      setCheck({ state: "ok", count: available.length });
    } catch (error) {
      setCheck({ state: "error", message: errorMessage(error) });
    }
  };

  return (
    <>
      <Field label="Provider">
        {(control) => (
          <select {...control} value={value.provider} onChange={(event) => choose(event.target.value as ProviderId)}>
            {providers
              .filter((candidate) => supportsRole(candidate, providerRole))
              .map((candidate) => (
                <option key={candidate.id} value={candidate.id}>
                  {candidate.label}
                </option>
              ))}
          </select>
        )}
      </Field>
      {preset && <ApiKeyField preset={preset} providerRole={providerRole} />}
      <Field
        label="Model"
        hint={<span className={check.state === "error" ? "is-error" : undefined}>{describeCheck(check)}</span>}
      >
        {(control) => (
          <span className="inline">
            <input
              {...control}
              list={modelListId}
              value={value.model}
              placeholder="Model name"
              spellCheck={false}
              onChange={(event) => onChange({ ...value, model: event.target.value })}
            />
            <button type="button" disabled={check.state === "checking"} onClick={() => void checkConnection()}>
              Check
            </button>
            <datalist id={modelListId}>
              {models.map((model) => (
                <option key={model} value={model} />
              ))}
            </datalist>
          </span>
        )}
      </Field>
      {LOCAL_PROVIDERS.includes(value.provider) ? (
        <BaseUrlField value={value} onChange={onChange} />
      ) : (
        <details className="advanced">
          <summary>Advanced</summary>
          <BaseUrlField value={value} onChange={onChange} />
        </details>
      )}
    </>
  );
}

function describeCheck(check: Check): string {
  switch (check.state) {
    case "idle":
      return "Check lists the models your key can use.";
    case "checking":
      return "Checking…";
    case "ok":
      return `Connected. ${check.count} models available; pick one from the list.`;
    case "error":
      return check.message;
  }
}

function BaseUrlField({ value, onChange }: { value: Endpoint; onChange: (endpoint: Endpoint) => void }) {
  return (
    <Field label="Base URL" hint="Requests go to this address. It must speak the provider's API.">
      {(control) => (
        <input
          {...control}
          value={value.baseUrl}
          placeholder="https://example.com/v1"
          spellCheck={false}
          onChange={(event) => onChange({ ...value, baseUrl: event.target.value })}
        />
      )}
    </Field>
  );
}
