declare module "jsdom" {
  export type DOMWindow = Window & typeof globalThis;

  export class VirtualConsole {
    on(event: "jsdomError", listener: (error: Error) => void): this;
  }

  export class JSDOM {
    constructor(
      html: string,
      options?: {
        url?: string;
        runScripts?: "dangerously";
        virtualConsole?: VirtualConsole;
        beforeParse?: (window: DOMWindow) => void;
      },
    );
    readonly window: DOMWindow;
  }
}
