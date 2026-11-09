import { expect, it } from "vitest";
import { displayHost, initials } from "./site";

it("derives readable monograms from site names", () => {
  expect(initials("ACME")).toBe("AC");
  expect(initials("Université de Lyon")).toBe("UD");
  expect(initials("moodle-campus")).toBe("MC");
  expect(initials("  ")).toBe("?");
});

it("shows the host and subdirectory without scheme", () => {
  expect(displayHost("https://moodle.example.edu/")).toBe("moodle.example.edu");
  expect(displayHost("https://example.edu/moodle/")).toBe("example.edu/moodle");
  expect(displayHost("not a url")).toBe("not a url");
});
