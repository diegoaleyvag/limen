import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import contract from "../../brand/contract-manifest.json";
import schema from "../../brand/five-decisions-project.schema.json";
import manifest from "../../portfolio.project.json";

type JsonValue =
  | undefined
  | boolean
  | null
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };
type JsonSchema = { [key: string]: JsonValue };

function isObject(value: JsonValue): value is { [key: string]: JsonValue } {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function matchesType(value: JsonValue, type: JsonValue): boolean {
  if (typeof type !== "string") return false;
  if (type === "object") return isObject(value);
  if (type === "array") return Array.isArray(value);
  if (type === "null") return value === null;
  return typeof value === type;
}

function validate(value: JsonValue, rule: JsonSchema, path: string, errors: string[]): void {
  if ("const" in rule && value !== rule.const)
    errors.push(`${path} must equal ${String(rule.const)}`);
  if ("enum" in rule && Array.isArray(rule.enum) && !rule.enum.includes(value)) {
    errors.push(`${path} is not an allowed value`);
  }

  if ("type" in rule && !matchesType(value, rule.type)) {
    errors.push(`${path} has the wrong type`);
    return;
  }

  if (typeof value === "string") {
    if (typeof rule.minLength === "number" && value.length < rule.minLength) {
      errors.push(`${path} is shorter than ${rule.minLength} characters`);
    }
    if (typeof rule.maxLength === "number" && value.length > rule.maxLength) {
      errors.push(`${path} is longer than ${rule.maxLength} characters`);
    }
    if (typeof rule.pattern === "string" && !new RegExp(rule.pattern).test(value)) {
      errors.push(`${path} does not match ${rule.pattern}`);
    }
    if (rule.format === "uri") {
      try {
        new URL(value);
      } catch {
        errors.push(`${path} is not a URI`);
      }
    }
  }

  if (Array.isArray(value) && isObject(rule.items)) {
    value.forEach((item, index) => {
      validate(item, rule.items as JsonSchema, `${path}[${index}]`, errors);
    });
  }

  if (isObject(value)) {
    if (Array.isArray(rule.required)) {
      for (const key of rule.required) {
        if (typeof key === "string" && !(key in value)) errors.push(`${path}.${key} is required`);
      }
    }
    const properties = isObject(rule.properties) ? rule.properties : {};
    if (rule.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!(key in properties)) errors.push(`${path}.${key} is not allowed`);
      }
    }
    for (const [key, propertyRule] of Object.entries(properties)) {
      if (key in value && isObject(propertyRule)) {
        validate(value[key], propertyRule, `${path}.${key}`, errors);
      }
    }
  }

  if (Array.isArray(rule.oneOf)) {
    const branchErrors = rule.oneOf.map((branch) => {
      const candidateErrors: string[] = [];
      if (isObject(branch)) validate(value, branch, path, candidateErrors);
      return candidateErrors;
    });
    const validBranches = branchErrors.filter((candidateErrors) => candidateErrors.length === 0);
    if (validBranches.length !== 1) errors.push(`${path} must match exactly one schema branch`);
  }
}

function validationErrors(value: JsonValue): string[] {
  const errors: string[] = [];
  validate(value, schema as JsonSchema, "$", errors);
  return errors;
}

describe("Five Decisions portfolio manifest", () => {
  it("matches the vendored exact schema and keeps demo/methodology links conservative", () => {
    expect(validationErrors(manifest as JsonValue)).toEqual([]);
    expect(manifest.capabilities.every((capability) => typeof capability === "object")).toBe(true);
    expect(manifest.evidence.every((entry) => typeof entry.reference === "string")).toBe(true);
    expect(manifest.links).toEqual({
      repository: "https://github.com/diegoaleyvag/limen",
      demo: null,
      methodology: null,
    });
  });

  it("rejects the pre-contract string capability shape", () => {
    const legacyManifest = {
      ...manifest,
      capabilities: ["context budgeting"],
    };
    expect(validationErrors(legacyManifest as unknown as JsonValue)).not.toEqual([]);
  });

  it("pins each vendored source to its frozen contract hash", () => {
    for (const source of Object.values(contract.sources)) {
      const bytes = readFileSync(resolve(process.cwd(), "..", "brand", source.path));
      const digest = createHash("sha256").update(bytes).digest("hex");
      expect(digest).toBe(source.sha256);
    }
  });
});
