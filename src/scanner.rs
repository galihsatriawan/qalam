use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stack {
    Rust,
    Go,
    Node,
    Python,
    Java,
    Kotlin,
    Grpc,
    Docker,
}

impl Stack {
    pub fn name(&self) -> &'static str {
        match self {
            Stack::Rust => "rust",
            Stack::Go => "go",
            Stack::Node => "node",
            Stack::Python => "python",
            Stack::Java => "java",
            Stack::Kotlin => "kotlin",
            Stack::Grpc => "grpc",
            Stack::Docker => "docker",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Stack::Rust => "Rust — memory safety, async (tokio), cargo ecosystem",
            Stack::Go => "Go — microservices, standard library patterns, go modules",
            Stack::Node => "Node/TypeScript — npm ecosystem, async/await, type safety",
            Stack::Python => "Python — scripting, data, FastAPI/Django patterns",
            Stack::Java => "Java — Spring Boot, Maven/Gradle, enterprise patterns",
            Stack::Kotlin => "Kotlin — JVM, coroutines, idiomatic Kotlin patterns",
            Stack::Grpc => "gRPC — protobuf contracts, service definitions",
            Stack::Docker => "Docker — containerization, multi-stage builds, compose",
        }
    }

    pub fn context_md(&self) -> String {
        match self {
            Stack::Rust => rust_context(),
            Stack::Go => go_context(),
            Stack::Node => node_context(),
            Stack::Python => python_context(),
            Stack::Java => java_context(),
            Stack::Kotlin => kotlin_context(),
            Stack::Grpc => grpc_context(),
            Stack::Docker => docker_context(),
        }
    }
}

/// Detect tech stacks from the files present in `root`.
pub fn detect(root: &Path) -> Vec<Stack> {
    let mut stacks = Vec::new();

    let markers: &[(&str, Stack)] = &[
        ("Cargo.toml", Stack::Rust),
        ("go.mod", Stack::Go),
        ("package.json", Stack::Node),
        ("pyproject.toml", Stack::Python),
        ("setup.py", Stack::Python),
        ("requirements.txt", Stack::Python),
        ("pom.xml", Stack::Java),
        ("build.gradle", Stack::Java),
        ("build.gradle.kts", Stack::Kotlin),
        ("Dockerfile", Stack::Docker),
        ("docker-compose.yml", Stack::Docker),
        ("docker-compose.yaml", Stack::Docker),
    ];

    for (marker, stack) in markers {
        if root.join(marker).exists() && !stacks.contains(stack) {
            stacks.push(stack.clone());
        }
    }

    // Proto files anywhere in tree → gRPC
    if has_proto_files(root) && !stacks.contains(&Stack::Grpc) {
        stacks.push(Stack::Grpc);
    }

    stacks
}

fn has_proto_files(root: &Path) -> bool {
    walk_for_extension(root, "proto", 3)
}

fn walk_for_extension(dir: &Path, ext: &str, depth: usize) -> bool {
    if depth == 0 {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                return true;
            }
        } else if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "node_modules" || name == "vendor" {
                continue;
            }
            if walk_for_extension(&path, ext, depth - 1) {
                return true;
            }
        }
    }
    false
}

fn rust_context() -> String {
    "# Skill: Rust\n\
    \n\
    ## Code Patterns\n\
    - Use `?` for error propagation with `anyhow::Result`\n\
    - Prefer `tokio` for async runtime\n\
    - Use `serde` + `serde_json`/`serde_yaml` for serialization\n\
    - Struct fields: snake_case. Types: PascalCase\n\
    - No unnecessary `.clone()` — prefer borrows where possible\n\
    \n\
    ## Agent Instructions\n\
    - Default to `anyhow::Result<()>` for fallible functions\n\
    - Use `tokio::spawn` for background tasks, not `std::thread`\n\
    - Cargo workspace: check `Cargo.toml` for existing dependencies before adding new ones\n\
    - Run `cargo clippy` before suggesting the code is complete\n\
    \n\
    ## Examples\n\
    ```rust\n\
    pub async fn run() -> anyhow::Result<()> {\n\
        let content = std::fs::read_to_string(\"file.txt\")?;\n\
        Ok(())\n\
    }\n\
    ```\n".to_string()
}

fn go_context() -> String {
    "# Skill: Go\n\
    \n\
    ## Code Patterns\n\
    - Explicit error handling: `if err != nil { return err }`\n\
    - Package names: short, lowercase, single word\n\
    - Interfaces: defined where used, not where implemented\n\
    - Use `context.Context` as first param for all I/O functions\n\
    - Struct embedding for composition\n\
    \n\
    ## Agent Instructions\n\
    - Check `go.mod` for module name and existing dependencies\n\
    - Use `go vet` and `golangci-lint` patterns\n\
    - Table-driven tests with `t.Run`\n\
    - Avoid naked returns\n\
    \n\
    ## Examples\n\
    ```go\n\
    func (s *Service) GetUser(ctx context.Context, id string) (*User, error) {\n\
        if id == \"\" {\n\
            return nil, errors.New(\"id is required\")\n\
        }\n\
        return s.repo.Find(ctx, id)\n\
    }\n\
    ```\n".to_string()
}

fn node_context() -> String {
    "# Skill: Node/TypeScript\n\
    \n\
    ## Code Patterns\n\
    - Prefer TypeScript strict mode\n\
    - Use `async/await` over raw Promise chains\n\
    - Zod or io-ts for runtime validation at boundaries\n\
    - Named exports preferred over default exports\n\
    \n\
    ## Agent Instructions\n\
    - Check `package.json` for existing deps before adding\n\
    - Use the project's existing test runner (jest/vitest)\n\
    - Follow ESLint config in the repo\n\
    \n\
    ## Examples\n\
    ```ts\n\
    export async function getUser(id: string): Promise<User> {\n\
        const user = await db.users.findUnique({ where: { id } });\n\
        if (!user) throw new NotFoundError(`User ${id} not found`);\n\
        return user;\n\
    }\n\
    ```\n".to_string()
}

fn python_context() -> String {
    "# Skill: Python\n\
    \n\
    ## Code Patterns\n\
    - Type hints on all public functions\n\
    - Pydantic for data validation\n\
    - `async def` for I/O-bound operations\n\
    - Dataclasses or Pydantic models instead of dicts\n\
    \n\
    ## Agent Instructions\n\
    - Check `pyproject.toml` or `requirements.txt` for existing deps\n\
    - Follow PEP 8, use `ruff` for linting\n\
    - Use `pytest` for tests\n".to_string()
}

fn java_context() -> String {
    "# Skill: Java\n\
    \n\
    ## Code Patterns\n\
    - Spring Boot idioms: `@Service`, `@Repository`, `@RestController`\n\
    - Constructor injection over field injection\n\
    - Use records for DTOs (Java 16+)\n\
    - Optional instead of null returns\n\
    \n\
    ## Agent Instructions\n\
    - Check `pom.xml` or `build.gradle` for Spring Boot version\n\
    - Follow existing package structure\n\
    - JUnit 5 + Mockito for tests\n".to_string()
}

fn kotlin_context() -> String {
    "# Skill: Kotlin\n\
    \n\
    ## Code Patterns\n\
    - Data classes for DTOs\n\
    - Extension functions for utility logic\n\
    - Coroutines (`suspend fun`) for async\n\
    - Sealed classes for result types\n\
    \n\
    ## Agent Instructions\n\
    - Prefer idiomatic Kotlin over Java-style Kotlin\n\
    - Use `Result<T>` or sealed classes over exceptions in domain logic\n\
    - Check `build.gradle.kts` for Kotlin version and coroutines dep\n".to_string()
}

fn grpc_context() -> String {
    "# Skill: gRPC / Protobuf\n\
    \n\
    ## Code Patterns\n\
    - Proto files define the contract — code is generated, not handwritten\n\
    - Version your packages: `package myservice.v1;`\n\
    - Use `google.protobuf.Timestamp` for times, not strings\n\
    - Field numbers are permanent — never reuse a deleted field number\n\
    \n\
    ## Agent Instructions\n\
    - Always check existing `.proto` files before adding new messages\n\
    - Run `buf lint` or `protoc` to validate after changes\n\
    - Generated code goes in a separate directory, never edit it manually\n\
    - Breaking changes require a new package version (v2, v3)\n".to_string()
}

fn docker_context() -> String {
    "# Skill: Docker\n\
    \n\
    ## Code Patterns\n\
    - Multi-stage builds: builder stage + minimal runtime image\n\
    - Non-root user in production images\n\
    - `.dockerignore` to exclude `target/`, `node_modules/`, `.git/`\n\
    - Pin base image versions (`alpine:3.20`, not `alpine:latest`)\n\
    \n\
    ## Agent Instructions\n\
    - Check existing `Dockerfile` before suggesting changes\n\
    - Keep layers cache-friendly: copy dependency files first, then source\n\
    - Use `docker compose` for local dev setup\n".to_string()
}
