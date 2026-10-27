// @vitest-environment happy-dom
import { describe, it, expect } from "vitest";
import { messages } from "../../i18n";

describe("Restart Policy i18n and UI constraints", () => {
  it("provides concise segmented control labels in zh-CN (<= 6 chars)", () => {
    const zh = messages["zh-CN"];
    expect(zh.restartNo.length).toBeLessThanOrEqual(6);
    expect(zh.restartAlways.length).toBeLessThanOrEqual(6);
    expect(zh.restartOnFailure.length).toBeLessThanOrEqual(6);

    expect(zh.restartNo).toBe("不重启");
    expect(zh.restartAlways).toBe("始终重启");
    expect(zh.restartOnFailure).toBe("仅异常退出");

    // Descriptions should provide full clear explanations
    expect(zh.restartNoDesc).toBeTruthy();
    expect(zh.restartAlwaysDesc).toBeTruthy();
    expect(zh.restartOnFailureDesc).toBeTruthy();
  });

  it("provides concise segmented control labels in en-US", () => {
    const en = messages["en-US"];
    expect(en.restartNo).toBe("No Restart");
    expect(en.restartAlways).toBe("Always");
    expect(en.restartOnFailure).toBe("On Failure");

    expect(en.restartNoDesc).toBeTruthy();
    expect(en.restartAlwaysDesc).toBeTruthy();
    expect(en.restartOnFailureDesc).toBeTruthy();
  });
});
