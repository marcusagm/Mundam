Com base na análise técnica dos arquivos fornecidos (`formats.rs`, `mod.rs`, `ffmpeg.rs`, `thumbnail_worker.rs`, etc.) e no plano de arquitetura **Unified Media Detection System (UMDS)**, aqui está a verificação profunda do estado atual do projeto:

### 1. O que já foi implementado

* **Single Source of Truth (Fonte Única de Verdade):** O arquivo `src-tauri/src/formats.rs` foi consolidado como o registro mestre de todos os formatos suportados, categorias (`MediaType`) e estratégias de miniatura.
* **Detecção Inteligente (Magic Bytes):** A função `FileFormat::detect` agora prioriza a leitura dos primeiros **8KB** do arquivo para identificação real via assinatura binária (utilizando a crate `infer`), com fallback para extensão de arquivo.
* **Wrapper do FFmpeg Robusto:** O arquivo `ffmpeg.rs` implementa a busca inteligente pelo binário (recursos integrados, pasta de desenvolvimento ou PATH do sistema) e o comando de extração de frame único (`-vframes 1`) com redimensionamento Lanczos.
* **Pipeline de Thumbnails Assíncrono:** O `thumbnail_worker.rs` gerencia uma fila de processamento em segundo plano usando `rayon` para paralelismo, limitando a carga da CPU e emitindo eventos `thumbnail:ready` para o frontend.
* **Estratégias de Extração Diversificadas:**
* **NativeImage:** Para formatos comuns (JPG, PNG, WebP).
* **Ffmpeg:** Para vídeos e RAWs (Canon, Nikon, Sony, Adobe DNG).
* **ZipPreview:** Para arquivos empacotados como Affinity Design e Clip Studio Paint.
* **Webview:** Para arquivos vetoriais (SVG).



### 2. Pendências e Inconsistências Identificadas

* **Desalinhamento no Indexador:** O arquivo `src-tauri/src/indexer/metadata.rs` ainda utiliza uma lógica simplificada (extensão ou `imagesize`) para detectar o formato, em vez de consumir o novo sistema `FileFormat::detect`. Isso cria uma divergência entre o formato salvo no banco de dados e o formato detectado pelo worker de thumbnails.
* **Estratégia "Webview" Incompleta:** Embora o `formats.rs` defina a estratégia `Webview` para SVGs, o roteamento em `thumbnails/mod.rs` redireciona essa estratégia para `icon::generate_thumbnail_icon`, resultando em ícones genéricos em vez de previews reais via WebView.
* **Formatos de Design Limitados:** O suporte para Adobe Illustrator (`.ai`, `.eps`) está marcado apenas como `Icon` no registro mestre. Arquivos `.ai` modernos baseados em PDF poderiam ser processados pelo FFmpeg ou extraídos via stream.
* **Falta do Comando Bridge:** O comando Tauri `get_supported_formats` proposto no plano original não foi encontrado na implementação de `lib.rs` ou arquivos de comando, o que significa que o frontend ainda pode estar usando dados estáticos ou hardcoded.

### 3. Melhorias Necessárias (Code Smell & Performance)

* **Duplicidade de Leitura de Header:** Atualmente, o worker detecta o formato (lendo 8KB) e, em seguida, o FFmpeg ou o decodificador nativo abre o arquivo novamente. Para arquivos pequenos, o impacto é mínimo, mas para milhares de arquivos em rede, isso pode ser otimizado.
* **Fallback de Erro Genérico:** No `thumbnail_worker.rs`, falhas na geração de miniaturas resultam apenas em logs no console. O sistema deveria marcar no banco de dados que aquele arquivo falhou, para evitar tentativas infinitas de processamento em cada reinicialização (poison pill).

### 4. Próximos Passos Recomendados

1. **Sincronização do Indexador:** Atualizar `indexer/metadata.rs` para utilizar `crate::formats::FileFormat::detect` ao preencher o campo `format` no banco de dados.
2. **Exposição do Registro para o Frontend:** Implementar o comando `get_supported_formats` no Tauri para que o frontend carregue dinamicamente as categorias de filtro (`Images`, `Videos`, etc.) baseadas no Rust.
3. **Refinamento do Suporte FFmpeg:** Expandir a lista de formatos profissionais no `formats.rs` (como MXF, Prores, EXR) para aproveitar a capacidade total detalhada na análise de thumbnails.
4. **Implementação da estratégia Webview:** Criar o módulo real para converter SVGs e possivelmente PDFs/HTMLs em miniaturas WebP usando uma janela oculta do WebView ou biblioteca especializada.
5. **Sistema de "Retry" no Worker:** Adicionar uma coluna `thumbnail_status` (Pending, Processing, Error, Success) e uma contagem de tentativas no banco de dados para gerenciar arquivos corrompidos que travam o processamento.



---

acho que cometeu um engano, pois get_supported_formats foi renomeado para get_library_supported_formats

---

Agradeço a correção! Você tem toda a razão. Eu cometi um equívoco ao referenciar o nome antigo do comando.

Ao verificar os arquivos atuais, o comando foi de fato implementado como `get_library_supported_formats` no arquivo `src-tauri/src/format_commands.rs` e devidamente registrado no `src-tauri/src/lib.rs`.

Aqui está a análise atualizada do estado da implementação com base nessa correção:

---

### Verificação da Implementação (Correção de Contexto)

* **Ponte Backend-Frontend:** O comando `get_library_supported_formats` está exposto corretamente no Backend. Ele retorna o `SUPPORTED_FORMATS` definido em `formats.rs`.
* **Consumo no Frontend:** No arquivo `src/core/store/systemStore.ts`, a lógica de inicialização deve invocar este comando exato para garantir que a "Fonte Única da Verdade" (Single Source of Truth) seja estabelecida no carregamento da aplicação.
* **Sincronização de Metadados:** O arquivo `src-tauri/src/indexer/metadata.rs` ainda parece utilizar uma lógica interna baseada em extensões e na crate `imagesize` para extrair dimensões e formato. Como observado anteriormente, isso ainda representa uma pendência de refatoração para que ele utilize o `FileFormat::detect` e mantenha a consistência com o restante do sistema.

---

### Próximos Passos (Revisados)

1. **Refatorar `get_image_metadata`:** Alterar a lógica em `src-tauri/src/indexer/metadata.rs` para que o campo `format` seja preenchido através do resultado de `FileFormat::detect(path)`.
2. **Validação da Estratégia Webview:** Corrigir o mapeamento em `src-tauri/src/thumbnails/mod.rs` para que arquivos com a estratégia `Webview` (como SVGs) não caiam no fallback de `icon::generate_thumbnail_icon`.
3. **Melhoria no Worker de Thumbnails:** No arquivo `src-tauri/src/thumbnail_worker.rs`, implementar um mecanismo que identifique falhas persistentes (arquivos corrompidos) para evitar que o worker tente reprocessar o mesmo arquivo em loop a cada inicialização.
4. **Expansão de Formatos Profissionais:** Adicionar ao `formats.rs` suporte explícito para formatos de vídeo de alta fidelidade citados anteriormente (como MXF e ProRes), garantindo que a estratégia seja definida como `Ffmpeg`.

Deseja que eu ajude com a refatoração do `metadata.rs` para integrar o novo sistema de detecção?



Com base na análise profunda dos arquivos e na correção sobre o comando `get_library_supported_formats`, aqui está o roteiro detalhado dos próximos passos para consolidar a arquitetura **Unified Media Detection System (UMDS)**:

---

### 1. Sincronização do Indexador (Backend)

O indexador atual ainda não utiliza a "Fonte Única da Verdade" para identificar arquivos.

* **Refatorar `get_image_metadata`:** Alterar o arquivo `src-tauri/src/indexer/metadata.rs` para substituir a detecção baseada em extensões e `imagesize` pelo método centralizado `FileFormat::detect(path)`.
* **Consistência de Dados:** Garantir que o campo `format` salvo no banco de dados SQLite corresponda ao `name` ou `mime_type` definido no registro mestre de formatos.

### 2. Implementação e Correção de Estratégias (Thumbnails)

Existem lacunas no roteamento de extração que precisam ser fechadas.

* **Ativar a Estratégia `Webview`:** No arquivo `src-tauri/src/thumbnails/mod.rs`, criar a lógica real para processar SVGs. Atualmente, eles estão sendo desviados para ícones genéricos (`icon::generate_thumbnail_icon`).
* **Aprimorar suporte a Design (AI/PSD):** * Atualizar o `formats.rs` para tentar extrair previews de arquivos `.ai` (que frequentemente possuem um PDF interno legível pelo FFmpeg).
* Validar a extração da primeira camada de arquivos PSD via FFmpeg.



### 3. Robustez do Worker de Processamento

Evitar desperdício de recursos com arquivos problemáticos.

* **Sistema de "Poison Pill":** Adicionar um controle no banco de dados (ex: coluna `retry_count` na tabela de imagens) para identificar arquivos que causam erro sistemático no FFmpeg ou decodificadores nativos, impedindo tentativas infinitas.
* **Tratamento de Erros no Worker:** No `thumbnail_worker.rs`, garantir que falhas críticas de processamento emitam uma notificação de erro específica para o frontend via evento Tauri.

### 4. Integração Final com o Frontend

Garantir que a interface seja 100% dinâmica.

* **Consumo do Comando de Formatos:** No `systemStore.ts` do frontend, garantir que a chamada para `get_library_supported_formats` seja o gatilho para popular os filtros de pesquisa e as extensões permitidas na interface.
* **Sincronização de Tipos:** Validar se o enum `MediaType` do Rust está perfeitamente alinhado com as interfaces TypeScript para evitar erros de cast no `invoke`.

---

### Resumo de Prioridades

| Prioridade | Tarefa | Arquivo Alvo |
| --- | --- | --- |
| **Alta** | Integrar `FileFormat::detect` no indexador | `indexer/metadata.rs` |
| **Alta** | Implementar render real de SVGs | `thumbnails/mod.rs` |
| **Média** | Adicionar controle de retentativas no worker | `thumbnail_worker.rs` |
| **Média** | Expandir lista de formatos profissionais | `formats.rs` |

**Gostaria que eu iniciasse a refatoração do `metadata.rs` para integrar o novo sistema de detecção agora?**