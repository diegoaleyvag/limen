// A single `aria-live="polite"` region announcing when results update or an error occurs, for
// screen-reader users who would otherwise have no signal that a re-render just happened.
import { el } from "./dom";

export interface LiveRegion {
  element: HTMLElement;
  announce: (message: string) => void;
}

/** Creates the live region element plus an `announce` function. Re-announcing the exact same
 * message twice in a row is given a trailing space toggle so assistive tech that only reacts to
 * text *changes* still fires (a common screen-reader-announcer gotcha). */
export function createLiveRegion(): LiveRegion {
  const element = el("div", {
    class: "visually-hidden",
    attrs: { "aria-live": "polite", role: "status" },
  });

  let previousMessage = "";
  const announce = (message: string): void => {
    const distinct = message === previousMessage ? `${message}\u00a0` : message;
    element.textContent = distinct;
    previousMessage = message;
  };

  return { element, announce };
}
