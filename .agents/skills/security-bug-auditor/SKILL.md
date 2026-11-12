---
name: security-bug-auditor
description: Audit source code, web applications, APIs, and desktop apps across any programming language to identify security vulnerabilities, logic bugs, and architectural weaknesses, then provide concrete code fixes. Use when the user asks to check, audit, pentest, review, or find vulnerabilities, security flaws, or bugs in their code, app, or website.
---
# Security & Bug Auditor

Audit source code, configurations, and application architecture across any programming language to uncover security vulnerabilities and logic bugs, providing actionable remediation and patched code.

## When to Use

- The user shares source code, project files, or architecture details and asks to find security holes, vulnerabilities, or bugs.
- The user wants a security review ("kiểm tra lỗ hổng", "audit code", "tìm bug bảo mật", "check bảo mật web/app") before deploying.
- The user asks how to harden or fix vulnerabilities in their web app, backend API, mobile app, or desktop software.

## Steps

1. **Identify Stack & Attack Surface**
   Determine the programming language, framework, runtime environment (web, API, desktop, mobile, cloud), and entry points where untrusted input enters the system (HTTP params, headers, files, IPC, CLI args, database reads).

2. **Perform Multi-Layer Vulnerability & Bug Analysis**
   Trace data flow from input sources to sensitive sinks and check against language-specific and universal flaw categories:

   - **Injection & Execution:** SQL/NoSQL Injection, Command Injection, XSS, SSTI, unsafe deserialization, or path traversal.
   - **Authentication & Authorization:** Broken access control (IDOR), missing middleware checks, weak JWT/session handling, hardcoded secrets/API keys, or privilege escalation.
   - **Data & Configuration Security:** Sensitive data exposure in logs/responses, permissive CORS/CSP headers, insecure storage/encryption, or exposed debug endpoints.
   - **Language & Runtime Specifics:**
     - *JavaScript/TypeScript (React, Node, etc.):* Prototype pollution, `dangerouslySetInnerHTML`, SSRF, unvalidated redirects, race conditions in async flows.
     - *Rust / C / C++:* Unsafe blocks, integer overflow, memory handling, unwrap/panic on untrusted input, unchecked FFI boundaries.
     - *Python / PHP / Java / C# / Go:* Type juggling, goroutine leaks, improper exception handling, insecure XML/YAML parsers.
   - **Logic & Edge-Case Bugs:** Race conditions (TOCTOU), off-by-one errors, unhandled null/error states, resource leaks, and business logic bypasses (e.g., negative quantities, payment validation skips).

3. **Report Findings by Severity**
   Group identified issues clearly from highest to lowest risk (`Critical`, `High`, `Medium`, `Low`). For each finding, provide:
   - **Location & Flaw:** Exact file, function, or line of code and the vulnerability type (e.g., OWASP category or CWE).
   - **Root Cause & Impact:** Explain clearly why the code is vulnerable and how the flaw could cause system failure or unauthorized access in theory.
   - **Concrete Remediation (Patched Code):** Provide the exact, drop-in replacement code in the user's language that fixes the bug securely.

4. **Provide Hardening Checklist**
   Conclude with 2-3 high-leverage architectural or configuration improvements specific to the user's stack (e.g., CSP rules, database RLS policies, compiler flags, or dependency auditing commands).

## Gotchas

- **Focus on Defensive Code Analysis & Fixes:** Never generate weaponized exploit payloads, automated attack scripts, or instructions for attacking live external targets. Explain the mechanics of a flaw strictly to clarify the root cause and motivate the code fix.
- **Avoid Generic Advice:** Always tie findings to the exact lines or patterns in the user's provided code rather than listing generic textbook definitions.
- **Preserve Functionality:** Ensure that the patched code maintains the application's intended behavior while closing the security gap.

