# utxo-blockchain

Protótipo educacional de uma blockchain baseada em UTXOs, escrito em Rust. O repositório reúne blocos com Proof of Work, transações assinadas com Ed25519, uma carteira de demonstração, persistência local e os primeiros tipos de mensagens para uma rede P2P.

> **Estado do projeto:** experimento em desenvolvimento. A implementação ainda não tem uma especificação de consenso completa, suíte de testes ou protocolo P2P integrado. Não use para armazenar fundos, chaves reais ou operar uma rede pública.

## Primeiros passos

### Requisitos

- Rust estável com suporte à edição 2024.
- A primeira compilação pode precisar baixar as dependências do crates.io.

### Compilar e executar

```sh
cargo check --locked
cargo run --locked -- 8080
```

O argumento opcional escolhe a porta do listener local; o padrão é `8080`. O programa inicia um listener em `127.0.0.1`, carrega `blockchain.json` do diretório atual (ou cria um bloco gênesis) e aguarda comandos no terminal. Pressione Enter para montar e minerar um bloco; digite `q` e pressione Enter para encerrar.

O arquivo `blockchain.json` é estado local gerado durante a execução e está ignorado pelo Git. Ele ainda não é uma fonte de dados confiável: a rotina atual de carregamento não revalida o snapshot completo. Consulte [as limitações e o desenho atual](docs/architecture.md) antes de experimentar.

## O que existe hoje

- Blocos com cabeçalho, nonce, dificuldade e raiz de Merkle.
- Transações com inputs/outputs, assinatura Ed25519 e validação básica contra o conjunto UTXO em memória.
- Recompensa de mineração e mempool em memória.
- Snapshot JSON da cadeia, do conjunto UTXO e da mempool.
- Esquema SQLite e operações auxiliares de armazenamento, ainda sem integração com o executável principal.
- Tipos de mensagem P2P e um listener inicial; a comunicação ainda não implementa o protocolo de mensagens definido no módulo `network`.

## Estrutura do repositório

| Caminho | Responsabilidade atual |
| --- | --- |
| `src/main.rs` | Inicialização interativa, carteira de demonstração, listener ad hoc e mineração local. |
| `src/models/` | Blocos, transações, estado da cadeia, persistência JSON e validação. |
| `src/crypto/` | Hashes, árvore/provas de Merkle e carteira Ed25519. |
| `src/db/` | Camada SQLite experimental, não usada pelo fluxo principal. |
| `src/network/` | Tipos de mensagem e um servidor inicial, ainda sem integração com o estado da cadeia. |
| `docs/architecture.md` | Fluxo executado hoje, módulos e limites conhecidos. |

## Limitações conhecidas

- A carteira usada pelo executável deriva uma chave de uma frase fixa no código. Isso serve apenas para demonstração; não é uma forma segura de gerar ou guardar chaves.
- As regras de consenso e a validação de inputs/blocos ainda estão incompletas. A validação completa da cadeia não reexecuta todas as regras sobre um estado UTXO reconstruído.
- A persistência JSON pode entrar em pânico com dados inválidos e confia em partes do snapshot carregado.
- A rede não usa framing de mensagens, limites de payload nem manipuladores integrados para blocos e transações.
- Não há testes automatizados nem CI no estado inicial deste roadmap.

## Roadmap

O trabalho está sendo organizado em issues pequenas e revisáveis:

1. [Documentar propósito, arquitetura e primeiros passos](https://github.com/Bappoz/utxo-blockchain/issues/1)
2. [Impor invariantes em transações e blocos](https://github.com/Bappoz/utxo-blockchain/issues/2)
3. [Tornar a persistência recuperável e consistente](https://github.com/Bappoz/utxo-blockchain/issues/3)
4. [Implementar protocolo P2P enquadrado e seguro](https://github.com/Bappoz/utxo-blockchain/issues/4)
5. [Estabelecer testes de regressão e CI Rust](https://github.com/Bappoz/utxo-blockchain/issues/5)

## Contribuição

Mudanças devem explicar o problema tratado e manter o escopo de cada pull request claro. Para comandos locais de compilação, use `cargo check --locked`; as issues de CI e cobertura automatizada estão no roadmap.

Leia [docs/architecture.md](docs/architecture.md) para entender o caminho atual dos dados e as fronteiras que ainda precisam ser concluídas.
