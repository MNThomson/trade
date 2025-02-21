# Trade

[<img alt="github" src="https://img.shields.io/badge/github-MNThomson/trade-bc3f48?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/MNThomson/trade)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/MNThomson/trade/ci.yml?branch=master&style=for-the-badge&logo=githubactions&logoColor=white" height="20">](https://github.com/MNThomson/trade/actions?query=branch%3Amaster)

## Production Deployment

> [!NOTE]
> **BEFORE** you run the following, make sure you have [`docker`](https://docs.docker.com/engine/install/) & the [`docker compose`](https://docs.docker.com/compose/install/#scenario-two-install-the-compose-plugin) plugin installed and running

```shell
git clone https://github.com/MNThomson/trade && cd trade

docker compose up --build
```

The server is now running (give it ~1min to build) and accessible at [http://localhost:3000/](http://localhost:3000/)

## Development Environment

> [!NOTE]
> You'll need [`rust`](https://www.rust-lang.org/learn/get-started) installed

```shell
cargo run  # Default development environment
cargo r    # Faster compile time with hot reloading (requires mold linker & cranelift backend)
cargo t    # Run the integration (correctness) test suite with file watching
cargo c    # Run the clippy linter with file watching
```

### Testing

GitHub Actions will automatically run the entire test suite on each commit as part of each service's workflow. These results can be viewed by clicking on the green checkmark besides the most recent commit on `master` (or alternatively viewed in the `Actions` tab). Run the command `cargo t` to kickoff the 1,000+ lines integration test suite.

## System Architecture

```mermaid
flowchart TD
    CB[Client Browser]
    CB -->|HTTP| R

    subgraph monolith_rect["Modular Monolith"]
        R{{Router}}
        R --> |HTTP /|F
        R --> |HTTP /authentication/*|UM
        R --> |HTTP /transaction/*|MM
        R --> |HTTP /engine/*|OM
        R --> |HTTP /setup/*|AM

        subgraph api_rect["Api Layer"]
            F(Frontend Module)
            UM(User Module)
            MM(Market Module)
            OM(Trade Module)
            AM(Admin Module)
        end
        style api_rect       stroke:#9b59b6,stroke-width:3px,fill:none,stroke-dasharray:4,4

        subgraph data_rect["Data Layer"]
        direction LR
            C[(Cache)] <---> D[(Database)]
            %%D <--> ME(Matching Engine)
        end
        style data_rect     stroke:#2ecc71,stroke-width:3px,fill:none,stroke-dasharray:4,4
        UM & MM & OM & AM --> data_rect


    end
    style monolith_rect stroke:#CE422B,stroke-width:3px,fill:none,stroke-dasharray:4,4
```
