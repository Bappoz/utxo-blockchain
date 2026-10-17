# Regras de consenso implementadas

Este documento registra o comportamento da validação atual, para que participantes do protótipo saibam quais blocos e transações serão aceitos. As regras são experimentais e ainda não formam um protocolo seguro para uma rede pública.

## Hashes e Proof of Work

- Transações e cabeçalhos de bloco são serializados com bincode e recebem SHA-256 duplo.
- A raiz de Merkle é calculada sobre as transações ordenadas dentro do bloco; quando um nível tem quantidade ímpar de hashes, o último é duplicado.
- O hash do cabeçalho precisa ter a quantidade exigida de bits zero à esquerda.
- O gênesis usa dificuldade zero. O primeiro bloco posterior ao gênesis precisa de pelo menos 16 bits de dificuldade e no máximo 256 bits. A dificuldade permanece igual nos blocos seguintes.
- A mineração retorna erro para dificuldades acima de 256 bits.

## Gênesis

- O hash anterior é o hash vazio e a dificuldade é zero.
- O bloco tem uma coinbase na primeira posição e nenhum outro tipo de transação.
- A soma das saídas da coinbase é exatamente `MINING_REWARD` (50 unidades).
- A tag de altura da coinbase é zero.

## Transações regulares

- Inputs e outputs não podem estar vazios.
- Cada input precisa apontar para um UTXO não gasto e ter uma assinatura Ed25519 válida para a chave pública daquele UTXO.
- Um mesmo outpoint `(tx_hash, output_index)` não pode aparecer duas vezes na mesma transação.
- Endereços dos outputs precisam codificar uma chave pública Ed25519 válida.
- As somas de inputs, outputs e taxas usam aritmética verificada; overflow rejeita a transação/bloco.
- O total de outputs não pode exceder o total de inputs. A diferença é a taxa da transação.
- Dentro de um bloco, transações são avaliadas em ordem sobre um UTXO set temporário. Um input gasto por uma transação anterior deixa de estar disponível para as seguintes.

## Coinbase e recompensa

- Um bloco precisa conter exatamente uma coinbase, na posição zero.
- A coinbase não possui assinatura nem input real; o campo `output_index` do input reservado contém a altura esperada do bloco.
- No gênesis, a recompensa é exatamente 50. Nos blocos seguintes, o total das saídas da coinbase pode ser menor que, ou igual a, `50 + taxas do bloco`; nunca pode exceder esse limite.
- A altura é parte do hash da coinbase, impedindo que recompensas idênticas de blocos diferentes sobrescrevam a mesma chave UTXO.

## Estado e replay

Ao aceitar um bloco, o nó remove cada UTXO consumido, insere as novas saídas e só então anexa o bloco à cadeia. A validação completa repete esse processo desde o gênesis e rejeita o primeiro bloco que falhar nas regras acima.

## Regras ainda ausentes

- Limites e política de timestamps.
- Ajuste de dificuldade por tempo/altura e regras de fork/escolha da cadeia.
- Maturidade de coinbase, tamanho máximo de bloco, limites de quantidade de transações e taxas mínimas.
- Separação de domínio/versão no formato de assinatura e serialização canônica com compatibilidade de versões.
- Protocolo de rede autenticado, sincronização entre nós e proteção contra spam.

Essas lacunas são parte do motivo pelo qual o projeto é educacional. Consulte as issues abertas para o plano de evolução.
