# Processamento compartilhado de fingerprints

**Implementado em:** 3 de novembro de 2026.

O scan manual de duplicados e o worker automático usavam implementações diferentes para calcular fingerprints. O caminho manual também executava leitura de arquivos e hashing dentro do adaptador de banco, com seleção e cancelamento diferentes do worker. Agora ambos usam o mesmo processador e a mesma política de tentativas.

## Responsabilidades

| Camada | Responsabilidade |
| --- | --- |
| [`DuplicateWorker`](../../src-tauri/src/processing/workers/duplicates/mod.rs) | Consumo contínuo, preferência de detecção automática e agendamento das comparações |
| [`DuplicateCommandService`](../../src-tauri/src/feature/duplicates/commands.rs) | Execução explícita, progresso, cancelamento, rebuild e fases de comparação |
| [`FingerprintProcessor`](../../src-tauri/src/feature/duplicates/fingerprint_processor.rs) | Seleção compartilhada, tentativa persistida, verificação da origem e gravação do resultado |
| [`processing/media/fingerprint.rs`](../../src-tauri/src/processing/media/fingerprint.rs) | Leitura do arquivo e extração técnica, em `spawn_blocking` |
| [`fingerprints.rs`](../../src-tauri/src/feature/duplicates/fingerprints.rs) | Matemática dos hashes perceptuais e resolução de pixels por providers |
| Repositories de processamento e duplicados | Elegibilidade SQL, tentativas e persistência |

O repository de duplicados não contém mais `rehash_pending_fingerprints`. O bootstrap injeta o mesmo repository de processamento nos caminhos manual e automático.

## Execução e resultados

1. Os chamadores obtêm admissão pelo `DuplicateProcessingCoordinator` e pelo `ProcessingControl` compartilhados. O processador não substitui esses controles.
2. A seleção consulta `ProcessingRepository::pending(Fingerprint, limit)`, excluindo assets removidos, na lixeira ou offline, retries futuros e falhas terminais. O scan manual trabalha sobre um snapshot fixo para manter o total de progresso estável.
3. Cada candidato inicia uma tentativa e verifica se a geração e a origem do catálogo continuam atuais.
4. O extrator abre o arquivo somente para leitura e calcula BLAKE3 incrementalmente em blocos de 8 KiB. O tamanho vem do mesmo handle aberto.
5. Para imagens, identificadas pela família e pelo formato, tenta decodificação direta, preview do provider e thumbnail de 1024 pixels, nessa ordem. Quando disponíveis, calcula dHash de 64 bits, hash espacial e dHash multiescala de 256 bits.
6. Antes da gravação, verifica novamente cancelamento e atualidade da tentativa. Somente um resultado salvo conclui a tentativa como sucesso.

Cancelamento é observado entre blocos de leitura, entre estratégias de extração e antes da persistência. Operações nativas já iniciadas continuam até retornar; não existe interrupção forçada. Cancelamento ou uma tentativa superada não publica um sucesso.

Um arquivo legível sem pixels decodificáveis mantém um fingerprint válido para **duplicatas exatas**. Campos visuais ficam vazios e esse resultado é considerado concluído. Isso evita que imagens sem extrator ou corrompidas retornem indefinidamente à fila apenas por não terem hash perceptual. Falhas ao abrir ou ler o original seguem a política persistida de até três tentativas, com backoff de cinco e trinta segundos.

## Compatibilidade e controles

- Os algoritmos e a representação dos hashes continuam na versão **5**. Não há migration nem rebuild obrigatório da biblioteca.
- Fingerprints v5 existentes continuam válidos. Resultados antigos sem hashes visuais não são recalculados automaticamente por esta mudança; o rebuild explícito permite recalculá-los, inclusive quando foram gerados com classificação incompleta da família.
- O scan manual respeita os mesmos retries futuros e falhas terminais do automático. Iniciar outro scan não contorna esse limite.
- O rebuild explícito apaga o índice derivado de duplicados e suas tentativas de fingerprints na mesma transação. Thumbnails, cores e seus registros de processamento são preservados.
- Pausa/retomada, preferência de detecção automática e coordenação de scans continuam nos chamadores. Não há novo comando IPC nem alteração de componente frontend.
- Nenhum original é alterado ou removido.

Exemplo do consumo técnico após a admissão pelos controles compartilhados:

```rust,ignore
let pending_assets = processor.pending(10).await?;
for asset_id in pending_assets {
    if cancellation_token.is_cancelled() {
        break;
    }
    processor.process_asset(&asset_id, &cancellation_token).await?;
}
```

O scan manual usa `processor.rehash_pending(&cancellation_token, Some(progress_sender))`. Falhas individuais ficam registradas para retry e não impedem o avanço dos demais candidatos.

## Limites e próxima etapa

Na etapa posterior, o processador passou a usar reserva persistente com heartbeat e publicação do fingerprint e do desfecho na mesma transação do Ledger. A implementação e seus limites estão em [Reservas persistentes e publicação transacional](processing-leases.md). A revisão ainda usa caminho, tamanho e modificação do catálogo; não garante snapshot físico atômico do original. O coordenador das comparações continua local à instância.

O canal de progresso manual continua não limitado. Matching exato e visual preservam seus algoritmos e adaptadores atuais; esta entrega unifica o cálculo dos fingerprints, sem migrar matching para jobs persistidos.

## Validação

No Windows, `./scripts/test-backend-windows.ps1 processing_` concluiu com **37 testes aprovados**. A suíte inclui equivalência dos resultados manual/automático, hashes v5 conhecidos, família de imagem com formato não reconhecido, leitura incremental, cancelamento, fallback preview/thumbnail, imagem corrompida com hash exato válido, origem superada, exclusão de assets inativos, retries e rebuild sem reiniciar outros estágios.

Os testes usam arquivos temporários e adaptadores SQLite reais. Não houve validação visual ou execução no macOS nesta etapa. O relatório histórico `docs/report/backend-architeture/definition/modules/05-job-scheduler-workers.md` não foi alterado.
