export class ConsoleDataError extends Error {
  readonly surface: string;
  readonly field: string;

  constructor(surface: string, field: string, detail?: string) {
    super(detail === undefined
      ? `${surface} is missing the authoritative field ${field}`
      : `${surface} is missing the authoritative field ${field}: ${detail}`);
    this.name = "ConsoleDataError";
    this.surface = surface;
    this.field = field;
  }
}
