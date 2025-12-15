# 🏗️ ARCHITECTURE-GUIDELINES  
### Package-Oriented Design — Inspired by William Kennedy (e Labs)

## 1. Overview

This document defines the **architectural conventions** for all projects adopting **Package-Oriented Design (POD)**.  
Its purpose is to align teams on **structure, dependency flow, and review rules**, ensuring every package has a clear purpose and place.

## 2. Design Philosophies

### 2.1 Purpose — “Provide, not Contain”
- A package must **provide** a single, coherent capability.  
- Names should describe what the package *provides* (e.g., `auth`, `json`, `orders`) — not generic labels like `common`, `helpers`, or `utils`.  
- If you struggle to name a package, the scope is unclear.  
- Avoid “dumping grounds” of unrelated functionality.  
*(ref: Ardan Labs – Design Philosophy on Packaging)*

### 2.2 Usability — Design for the Consumer
- The API must be **intuitive, minimal, and stable**.  
- Hide internal complexity; expose only what’s necessary.  
- Respect the consumer’s resource and performance constraints.  
- Prevent cascading changes by using interfaces to decouple contracts from implementations.  
- Avoid forcing consumers to type-assert concrete values.

### 2.3 Portability — Design for Reuse
- Keep dependencies minimal; prefer only the standard library.  
- Avoid embedding app-specific policy (e.g., logging, configuration, metrics).  
- Prevent creating a “single point of dependency” package that every module imports.  
- Strive for composable, policy-free code that can move between projects without change.

## 3. Project Structure

### 3.1 Repository Layout
Each company or organization should maintain:
- **One `kit/` project** — the reusable foundation (standard library extensions).  
- **Multiple `application/` projects** — self-contained deployable services.

### 3.2 Example Application Layout
```shell
github.com/org/project
├── cmd/ # Application entry points
│ ├── api/
│ │ └── main.go
│ └── worker/
│ └── main.go
├── internal/ # Project-specific business logic
│ ├── domain/
│ ├── services/
│ └── platform/ # Infra & integrations (DB, auth, I/O)
└── vendor/ # Vendored dependencies (including kit)
```

**Layer roles:**
- `cmd/` → Entry points (startup, config, orchestration)  
- `internal/` → Business logic & domain services  
- `internal/platform/` → Infra & platform adapters  
- `kit/` → Portable, reusable building blocks  

---

## 4. Validation Checklist

| # | Validation | Question |
|:-:|:-|:-|
| 1 | **Location** | Is the package placed in the correct layer? |
| 2 | **Dependencies** | Are imports minimal and one-directional? |
| 3 | **Policies** | Is logging/config only used where allowed? |
| 4 | **API Boundaries** | Are exported types clean (no 3rd-party leaks)? |
| 5 | **Error Handling** | Are error rules appropriate for the layer? |
| 6 | **Testing** | Are tests aligned with the package’s role? |
| 7 | **Naming** | Does the name reflect purpose and cohesion? |
| 8 | **Granularity** | Is the package cohesive and not oversized? |

---

## 5. Layer Rules

| Layer | Purpose | Allowed Imports | Policy | Error Rules | Testing Focus |
|:--|:--|:--|:--|:--|:--|
| **kit/** | Foundation (shared libs) | Stdlib only | ❌ No logging or policy | Return root cause only | Unit tests |
| **internal/platform/** | Infra adapters | Other platform pkgs | ❌ No policy | Return root cause only | Unit tests |
| **internal/** | Domain logic | `platform`, `kit` | ✅ May log or wrap | Minor handling | Unit + integration |
| **cmd/** | Entrypoints | All project packages | ✅ May set policy | May panic/recover | Integration tests |

---

## 6. Error and Panic Rules

| Layer | Panics | Wrap Errors | Handle Errors |
|:--|:--|:--|:--|
| kit | 🚫 Never | 🚫 Never | Return root cause |
| internal/platform | 🚫 Never | 🚫 Never | Return root cause |
| internal | 🚫 Never | ✅ Allowed | Handle selectively |
| cmd | ✅ Allowed | ✅ Allowed | Handle majority |

> Only the `cmd/` layer may recover panics, and only when full integrity can be restored.  
*(ref: Ardan Labs – Package Oriented Design)*

---

## 7. Testing Discipline

| Layer | Allowed Tools | Test Focus |
|:--|:--|:--|
| kit, internal, platform | Std `testing` package | Unit tests inside package |
| cmd | 3rd-party test tools allowed | Integration/system tests |

---

## 8. Naming & Granularity
- Avoid meaningless names (`common`, `shared`, `misc`, `helpers`).  
- Packages should be small enough to understand, large enough to be useful.  
- Use names that convey responsibility (`orders`, `auth`, `json`, `metrics`).  

---

## 9. Dependency & Cross-Import Rules
- Two packages **must never import each other** — merge or decouple instead.  
- Imports flow **downward only** (`cmd → internal → platform → kit`).  
- Use the folder structure to make dependency flow explicit.


## Rules of package structure

- kit 
  - Packages that provide foundational support for the different Application projects that exist.
  - logging, configuration or web functionality.
- cmd 
  - Packages that provide support for a specific program that is being built.
  - startup, shutdown and configuration.
- internal
  - Packages that provide support for the different programs the project owns.
  - CRUD, services or business logic.
- internal/platform/
  - Packages that provide internal foundational support for the project..
  - database, authentication or marshaling.

## Rules for dependency choices.

- All
  - Validate the cost/benefit of each dependency.
  - Question imports for the sake of sharing existing types.
  - Question imports to others packages at the same level.
  - If a package wants to import another package at the same level: 
    - Question the current design choices of these packages.
    - If reasonable, move the package inside the source tree for the package that wants to import it.
    - Use the source tree to show the dependency relationships.
- internal/
  - Packages from these locations CAN’T be imported:
    - cmd
- internal/platform/
  - Packages from these locations CAN’T be imported:
    - cmd/
    - internal/

## Validate the policies being imposed.
- Kit, internal/platform/
  - NOT allowed to set policy about any application concerns.
  - NOT allowed to log, but access to trace information must be decoupled.
  - Configuration and runtime changes must be decoupled.
  - Retrieving metric and telemetry values must be decoupled.
- cmd/, internal/
  - Allowed to set policy about any application concerns.
  - Allowed to log and handle configuration natively.

## Validate how data is accepted/returned.

- All
  - Validate the consistent use of value/pointer semantics for a given type.
  - When using an interface type to accept a value, the focus must be on the behavior that is required and not the value itself.
  - If behavior is not required, use a concrete type.
  - When reasonable, use an existing type before declaring a new one.
  - Question types from dependencies that leak into the exported API.
    - An existing type may no longer be reasonable to use.

## Rules to Error Handling 

- All 
  - Handling an error means:
    - The error has been logged.
    - The application is back to 100% integrity.
    - The current error is not reported any longer.
- kit 
  - NOT allowed to panic an application.
  - NOT allowed to wrap errors.
  - Return only root cause error values.
- cmd 
  - Allowed to panic an application.
  - Wrap errors with context if not being handled.
  - Majority of handling errors happen here.
- internal
  - NOT allowed to panic an application.
  - Wrap errors with context if not being handled.
  - Minority of handling errors happen here.
- internal/platform/
  - NOT allowed to panic an application.
  - NOT allowed to wrap errors.
  - Return only root cause error values.

## Rules to Recovering panics

- cmd 
  - Can recover any panic.
  - Only if system can be returned to 100% integrity.
- kit/, internal/, internal/platform/
  - Can not recover from panics unless:
    - Goroutine is owned by the package
    - Can provide an event to the app about the panic.

