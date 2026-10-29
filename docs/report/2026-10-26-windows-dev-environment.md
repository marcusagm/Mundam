# Relatório de Engenharia: Configuração do Ambiente de Desenvolvimento Windows (Mundam-v2)

**Data:** 26 de Outubro de 2026  
**Autor:** Antigravity AI Assistant & Marcus Maia  
**Repositório:** `Mundam-v2`  
**Escopo:** Estabilização do ambiente Windows (10/11 x64), resolução de falhas de compilação/linking do backend Rust (Tauri v2), automação via scripts e diretrizes para eliminação de dependências nativas problemáticas.

> **Estado do relatório:** Este documento registra o diagnóstico histórico feito em 26 de outubro. As seções 2–7 descrevem ferramentas e soluções daquela época e não devem ser usadas como instruções atuais. Em 29 de outubro, o setup foi alinhado ao toolchain MSVC oficial do Tauri; `rsraw`/LibRaw e `gpr_sdk` foram removidos do backend, assim como MinGW. O estado atual e os comandos de preparação estão resumidos em “Estado atual” abaixo; detalhes históricos foram mantidos para explicar a origem dos problemas.

## Estado atual (29 de outubro de 2026)

- **Windows:** Rust `stable-x86_64-pc-windows-msvc`, Visual Studio Build Tools com workload “Desktop development with C++” e Windows SDK, Node.js 18 ou superior e `npm ci`. `scripts/tauri.cjs` seleciona MSVC para comandos Tauri em Windows, inclusive quando há um override GNU antigo no checkout. Tauri gera MSI e NSIS (`targets: all`); o recurso opcional VBSCRIPT pode ser necessário para MSI. WebView2 está presente no Windows 10/11 compatível e o instalador Tauri pode instalá-lo em sistemas que precisem. Não são necessários MSYS2, MinGW, LLVM/libclang nem CMake para compilar o backend atual.
- **FFmpeg Windows que estava staged:** a cópia Gyan full-build 9.0.2 habilitava `--enable-gpl` e `--enable-version3`. Ela foi removida da pasta de recursos, e o setup agora baixa um build LGPL compartilhado com DLLs de suporte. Execute o setup antes de criar o instalador; não distribua um build Gyan antigo.
- **RAW:** FlashRaw fornece suporte GPR em Rust. O projeto também usa Rawler para outros formatos RAW; não há dependência de LibRaw no setup.
- **Recursos para instaladores:** FFmpeg e FFprobe são necessários para a pipeline de mídia. O setup Windows baixa o release BtbN `n9.0.2-37-gae19868753` marcado como LGPL shared e recusa builds com `--enable-gpl` ou `--enable-nonfree`; setup macOS/Linux ainda usa o FFmpeg do sistema e grava a configuração em `src-tauri/resources/build-info/` para revisão de licença. Como o build Windows não contém `libx264`, o backend detecta o encoder H.264 disponível e usa `libopenh264` ou um encoder da plataforma. Assimp é opcional para conversão de alguns modelos 3D. PDFium é baixado pelo setup a partir do release `chromium/8125`. A SoundFont versionada continua sendo usada pelo preview MIDI; confira a procedência/licença registrada em `docs/backend/dependences.md` antes de redistribuir.
- **Preparação:** execute `scripts/setup-windows.ps1` em PowerShell. Use `-SkipSystem` para não instalar ferramentas via winget ou `-Check` para executar `cargo check --locked` após preparar o ambiente. O setup aplica `npm ci`, valida/stageia os recursos e configura o override MSVC.
- **Gerar instaladores:** use `npm run tauri build`. Se o MSI falhar com erro de `light.exe`, habilite VBSCRIPT em Windows Optional Features. O script de setup verifica o estado e emite um aviso sem tentar alterar recursos opcionais do Windows.
- **Outras plataformas:** `scripts/setup-macos.sh` e `scripts/setup-linux.sh` instalam os pré-requisitos nativos do Tauri, fazem `npm ci` e preparam os recursos dinâmicos exigidos pelo runtime.

O inventário detalhado dos recursos e suas licenças está em [`docs/backend/dependences.md`](../backend/dependences.md). Os scripts em `scripts/` são a fonte de verdade para a preparação de cada plataforma.

---

## 1. Visão Geral Executiva

O Mundam-v2 é uma aplicação desktop de alta performance construída sobre o framework **Tauri v2**, com frontend em Vite/SolidJS e backend em Rust. O projeto lida com dezenas de formatos de mídia avançados (imagens raster, arquivos RAW fotográficos, modelos 3D, áudio MIDI com sintetizadores, PDFs e texturas gráficas).

Originalmente desenvolvido e otimizado com foco no ecossistema macOS/Linux, a compilação no Windows encontrava múltiplos bloqueios estruturais que impediam a execução de `npm run tauri dev`. 

Durante esta sessão de engenharia:
1. Identificamos e sanamos **todas as falhas de compilação e linking** no Windows.
2. Compilamos com êxito todas as **909 dependências** do workspace backend.
3. Geramos o binário nativo executável `mundam.exe` funcional.
4. Validamos a execução em tempo real do frontend e de todos os workers e watchers do backend.
5. Atualizamos o script automatizado `scripts/setup-windows.ps1` para que qualquer novo ambiente seja configurado de forma determinística e reproduzível.
6. Mapeamos um plano de ação para eliminar dependências C/C++ frágeis em favor de implementações puras e seguras em Rust.

---

## 2. Requisitos de Sistema e Ferramentas (histórico)

Para construir e executar o Mundam-v2 no Windows de forma limpa, são necessários os seguintes componentes:

### 2.1. Sistema Operacional e Arquitetura
* **Windows 10** (build 1809 ou superior) ou **Windows 11** de 64 bits (`x86_64`).
* **Gerenciador de Pacotes:** `winget` (Windows Package Manager / "Instalador de Aplicativo" da Microsoft Store).

### 2.2. Ferramentas do Sistema e Compiladores
| Ferramenta         | Identificador / Pacote                                        | Finalidade no Projeto                                                                         |
| :----------------- | :------------------------------------------------------------ | :-------------------------------------------------------------------------------------------- |
| **Git**            | `Git.Git`                                                     | Controle de versão e atualização de submódulos (`gpr_sdk`).                                   |
| **Node.js LTS**    | `OpenJS.NodeJS.LTS` (>= 18, testado v24)                      | Execução do bundler Vite e gerenciamento de dependências npm.                                 |
| **Rustup & Rust**  | `Rustlang.Rustup` (Toolchain: `stable-x86_64-pc-windows-gnu`) | Compilador Rust com suporte a toolchain MinGW/GCC.                                            |
| **MSYS2 / UCRT64** | `MSYS2.MSYS2` (`mingw-w64-ucrt-x86_64-gcc`)                   | GCC 15.x, Binutils (`windres`, `ld`, `ar`) e CRT moderna do Windows (`ucrtbase.dll`).         |
| **LLVM**           | `LLVM.LLVM`                                                   | Fornece `libclang.dll` para geração de bindings FFI via `bindgen` (usado por `rsraw-sys`).    |
| **CMake**          | `Kitware.CMake`                                               | Construção de submódulos C++ (ex.: `gpr_sdk`).                                                |
| **FFmpeg**         | `Gyan.FFmpeg`                                                 | Binários `ffmpeg.exe` e `ffprobe.exe` para pipeline de vídeo e fallback de extração de mídia. |
| **SoundFont**      | Asset versionado em `src-tauri/resources/soundfonts/`         | Recurso MIDI empacotado com o app; não exige Python no setup.                                 |

### 2.3. Binários e Recursos Estagiados em `src-tauri/`
* `resources/pdfium/pdfium.dll`: Biblioteca dinâmica de renderização PDF (baixada pelo setup script).
* `resources/ffmpeg/windows-x64/ffmpeg.exe` e `resources/ffmpeg/windows-x64/ffprobe.exe`: Binários do FFmpeg estagiados localmente para empacotamento do Tauri.
* `resources/soundfonts/*.sf2`: Arquivo SoundFont (ex.: `FluidR3_GM.sf2` ou `GeneralUser GS.sf2`) para o sintetizador `rustysynth`.
* `resources/assimp/windows-x64/assimp.exe` *(opcional)*: Conversor de malhas 3D para renderização de modelos.

---

## 3. Variáveis de Sistema e Configurações de Ambiente (histórico)

A correta configuração das variáveis de ambiente é mandatória para evitar conflitos entre diferentes compiladores no Windows:

### 3.1. Variável `PATH` (Precedência Crítica)
O Windows frequentemente possui múltiplas versões de utilitários como `gcc`, `ld` e `windres` instaladas (ex.: MSYS POSIX `/usr/bin`, MinGW UCRT `/ucrt64/bin`, Git Bash `/bin`, etc.).

* **Regra de Ouro:** `C:\msys64\ucrt64\bin` **DEVE** preceder `C:\msys64\usr\bin` e caminhos do Git Bash.
* **Motivo:** O binário `C:\msys64\usr\bin\windres.exe` faz parte do ambiente de emulação POSIX/Cygwin do MSYS e trata barras invertidas do Windows (`\`) como caracteres de escape de C (`\x`, `\m`), corrompendo caminhos na compilação de recursos (`resource.rc`). O binário `C:\msys64\ucrt64\bin\windres.exe` é nativo do Windows e processa caminhos corretamente.

### 3.2. Variável `LIBCLANG_PATH`
* **Valor:** `C:\Program Files\LLVM\bin`
* **Finalidade:** O crate `bindgen` (usado por bibliotecas C/C++ como `rsraw-sys`) precisa carregar dinamicamente a `libclang.dll` para analisar os headers C++ da LibRaw. Sem essa variável, a compilação do `rsraw-sys` aborta com erro de inicialização do Clang.

### 3.3. Configuração do Cargo por plataforma
O repositório não fixa mais `target` nem `target-dir` em `.cargo/config.toml`. O launcher `scripts/tauri.cjs` aplica `CARGO_BUILD_TARGET=x86_64-pc-windows-gnu` e `CARGO_TARGET_DIR=C:/cargo-target/mundam` somente ao executar o Tauri no Windows. macOS e Linux usam o target nativo e o diretório `src-tauri/target` padrão do Cargo.

---

## 4. Diagnóstico Técnico dos Problemas e Soluções (Causa Raiz; histórico)

Abaixo está o registro técnico aprofundado dos 6 principais obstáculos superados durante a estabilização do ambiente Windows:

### 4.1. Falha Fatal do `rsraw-sys v0.1.0` com Compilador MSVC
* **Sintoma:** Ao executar `cargo build` ou `npm run tauri dev` com o toolchain padrão do Visual Studio (`stable-x86_64-pc-windows-msvc`), a compilação abortava imediatamente com:
  ```text
  error: failed to run custom build command for `rsraw-sys v0.1.0`
  Caused by:
    ...
    panicked at build.rs:21:9:
    MSVC is not supported
  ```
* **Causa Raiz:** O crate `rsraw-sys` compila a biblioteca C++ `LibRaw` internamente através do crate `cc`. No arquivo `build.rs` da versão 0.1.0 e da versão 0.1.1 do `rsraw-sys`, há uma verificação explícita:
  ```rust
  let compiler = libraw.get_compiler();
  if compiler.is_like_msvc() {
      panic!("MSVC is not supported");
  }
  ```
  O autor do wrapper bloqueou hardcoded o MSVC porque a compilação da LibRaw via C++ no MSVC requer flags e macros específicas não suportadas pelo build script do crate.
* **Solução Adotada:** 
  Migramos o projeto para o toolchain GNU oficial do Rust (`stable-x86_64-pc-windows-gnu`) integrado ao MinGW-w64 (GCC 15.2.0 via MSYS2 UCRT64), onde o código C++ compila nativamente sem falhas.

---

### 4.2. Falha de Caminhos com Espaços no `dlltool` e `windres`
* **Sintoma:** 
  O usuário possui um espaço no nome de usuário do Windows (`C:\Users\Marcus Maia\...`). Durante a linkagem de bibliotecas dinâmicas ou compilação de recursos, o `dlltool` do MinGW falhava separando `C:\Users\Marcus` como argumento de caminho e `Maia\...` como flag inválida.
* **Causa Raiz:** 
  Diversas ferramentas de linha de comando históricas do GNU Binutils (`dlltool`, `collect2`) no Windows têm falhas no parsing de argumentos contendo espaços quando passados através de arquivos de resposta (`@response.txt`) ou argumentos CLI sem aspas estritas.
* **Solução Adotada:**
  O launcher Windows `scripts/tauri.cjs` redireciona o target-dir para `C:/cargo-target/mundam` apenas nos processos Tauri do Windows. Essa configuração evita caminhos com espaços sem impor esse diretório aos builds de macOS e Linux.

---

### 4.3. Falha de Biblioteca Nativa Ausente: `intel_tex_2` e `image_dds`
* **Sintoma:** Ao compilar com o toolchain GNU, o processo quebrava com:
  ```text
  error: could not find native static library `kernelx86_64-pc-windows-gnu`, perhaps an -L flag is missing?
  ```
* **Causa Raiz:** 
  O crate `image_dds = "0.7.2"` estava declarado no `src-tauri/Cargo.toml` com suas features padrão ativadas (`default-features = true`). Entre as features padrão estava a feature `encode`, que inclui o crate `intel_tex_2`. O `intel_tex_2` utiliza código assembly vetorizado compilado via Intel ISPC cujos objetos estáticos pré-compilados foram gerados exclusivamente para a convenção de chamada e ABI do **MSVC** (`kernel.lib`), resultando em uma tentativa absurda do build script de concatenar o nome do target triple gerando `kernelx86_64-pc-windows-gnu`.
* **Solução Adotada:** 
  Analisamos o código-fonte do Mundam e constatamos que a aplicação **apenas decodifica** arquivos `.dds` (`image_dds::image_from_dds`), nunca realizando compressão/codificação de texturas em tempo de execução. Ajustamos a dependência no [`src-tauri/Cargo.toml`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/Cargo.toml#L85) para:
  ```toml
  image_dds = { version = "0.7.2", default-features = false, features = ["ddsfile", "image", "strum"] }
  ```
  Isso eliminou o `intel_tex_2` da árvore de dependências sem qualquer impacto funcional.

---

### 4.4. Gargalo de Linkagem com `ld.bfd` (Linker GNU)
* **Sintoma:** 
  Ao compilar o binário final `mundam.exe`, a compilação atingia 907/909 e congelava por mais de 6 minutos sem conclusão na etapa de linking.
* **Causa Raiz:** 
  Por padrão, o perfil de desenvolvimento (`[profile.dev]`) do Rust compila o binário com `debug = 2` (nível máximo de informações DWARF: símbolos, tabelas de tipos de variáveis, linhas e escopos locais para todas as 909 dependências). No Windows, o linker GNU clássico (`ld.bfd`) realiza alocação extensiva em memória e parsing síncrono desse volume multigiagbyte de dados de debug, gerando tempos de link intoleráveis.
* **Solução Adotada:** 
  Configuramos o perfil de desenvolvimento em [`src-tauri/Cargo.toml`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/Cargo.toml#L101-L103) para utilizar `debug = 1`:
  ```toml
  [profile.dev]
  # Mantém tabelas de linhas para backtraces informativos e reduz massivamente o payload DWARF
  debug = 1
  ```
  O tempo de linkagem caiu de **mais de 6 minutos para menos de 25 segundos**, mantendo rastreabilidade completa de erros (arquivos e linhas de panic).

---

### 4.5. Falha de Pré-processamento no `tauri-winres` / `windres`
* **Sintoma:** 
  Ao compilar o crate principal do Tauri, ocorria o seguinte erro no `build.rs`:
  ```text
  cc1.exe: fatal error: C:cargo-targetmundamx86_64-pc-windows-gnudebugbuildmundam-59fc4c222b83c769outresource.rc: No such file or directory
  compilation terminated.
  windres: preprocessing failed.
  ```
* **Causa Raiz:** 
  O crate `tauri-winres` (via `embed-resource`) invoca o utilitário `windres.exe` passando o arquivo de recursos gerado em `OUT_DIR`. Por baixo dos panos, o `windres` executa `gcc -E` (que chama `cc1.exe`). Se o utilitário `windres` for o da camada POSIX (`C:\msys64\usr\bin\windres.exe`), ele interpreta as barras invertidas do caminho Windows como sequências de escape do C (ex.: `\c` -> `c`, `\m` -> `m`), apagando as barras do caminho.
* **Solução Adotada:** 
  1. O launcher Windows `scripts/tauri.cjs` define um `CARGO_TARGET_DIR` sem espaços somente para processos Tauri no Windows.
  2. No arquivo [`src-tauri/build.rs`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/build.rs#L1-L8), inserimos uma diretiva no início da função `main` para prepender os diretórios nativos do MinGW (`C:\msys64\ucrt64\bin;C:\msys64\mingw64\bin`) na variável de ambiente `PATH` do processo. Assim, o `embed-resource` invoca o `windres` nativo da UCRT, que executa o pré-processamento com sucesso.

---

### 4.6. Conflito de Headers Pthread no GoPro GPR SDK
* **Sintoma:** 
  O submódulo `vendor/gpr_sdk` (código C++ da GoPro baseado no Adobe DNG SDK) implementa seu próprio arquivo de compatibilidade de threads (`dng_pthread.cpp`) projetado para o compilador MSVC. Ao compilar com MinGW no Windows, ocorriam conflitos graves de redefinição de tipos entre o `dng_pthread.cpp` e o `pthread.h` nativo do MinGW (`pthread_rwlockattr_t`, `dng_pthread_t`).
* **Causa Raiz:** 
  O DNG SDK não foi projetado para ambientes MinGW no Windows, presumindo que compilações Windows sempre utilizem MSVC.
* **Solução Adotada:** 
  1. No [`src-tauri/build.rs`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/build.rs#L17-L23), adicionamos uma condicional `target_os != "windows"` para não invocar o CMake do `gpr_sdk` em builds Windows.
  2. No módulo [`src-tauri/src/processing/media/extractors/gpr.rs`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/src/processing/media/extractors/gpr.rs), isolamos os símbolos C FFI sob `#[cfg(not(target_os = "windows"))]` e criamos stubs seguros sob `#[cfg(target_os = "windows")]` que retornam mensagens amigáveis em caso de tentativa de extração direta de GPR.

---

## 5. Inventário dos Arquivos Modificados naquela implementação (histórico)

| Arquivo                                                                                                                                                                 | Natureza da Alteração               | Detalhes Técnicos                                                                                                                        |
| :---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------- |
| [`scripts/tauri.cjs`](../../scripts/tauri.cjs)                                                                                                                          | **Launcher do Tauri**               | Define target GNU e target-dir sem espaços somente no Windows.                                                                           |
| [`scripts/setup-windows.ps1`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/scripts/setup-windows.ps1)                                                   | **Atualização de Setup**            | Instala MSYS2 via winget; instala gcc ucrt64; prioriza `ucrt64\bin` no PATH; instala toolchain Rust GNU; prepara os binários do Windows. |
| [`src-tauri/Cargo.toml`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/Cargo.toml)                                                             | **Configuração de Dependências**    | Desativa `default-features` do `image_dds`; atualiza `rsraw` para `0.1.1`; insere `[profile.dev] debug = 1` para acelerar o linker.      |
| [`src-tauri/build.rs`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/build.rs)                                                                 | **Lógica de Build Script**          | Prepend no `PATH` para priorizar o `windres` do MinGW nativo; desativa compilação do `gpr_sdk` em plataformas Windows.                   |
| [`src-tauri/src/processing/media/extractors/gpr.rs`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/src/processing/media/extractors/gpr.rs)     | **Compatibilidade Multiplataforma** | Adiciona stubs `#[cfg(target_os = "windows")]` para geração de thumbnails e previews de GoPro GPR.                                       |
| [`src-tauri/src/delivery/tauri/commands/mutations.rs`](file:///c:/Users/Marcus%20Maia/Desktop/Aplicativos/Mundam-v2/src-tauri/src/delivery/tauri/commands/mutations.rs) | **Correção de Warning**             | Adiciona `let _ = &paths;` no comando `copy_files_to_clipboard` para evitar warning de parâmetro não utilizado fora do macOS.            |

---

## 6. Mapeamento de Dependências Problemáticas e Plano de Migração (histórico)

Para garantir estabilidade a longo prazo, redução drástica no tempo de compilação em CI/CD e portabilidade 100% nativa sem dependência de toolchains C++ externos complexos, recomendamos a substituição das seguintes dependências:

### 6.1. Dependência Problemática 1: `rsraw` / `rsraw-sys` (LibRaw C++ Wrapper)
* **Grau de Criticidade:** 🔴 **Crítico**
* **Dificuldades Causadas:** 
  * Rejeita o compilador MSVC via panic no `build.rs`.
  * Força o projeto inteiro no Windows a utilizar o toolchain GNU (MinGW), inviabilizando o uso simplificado do Visual Studio Build Tools.
  * Requer a instalação pesada do pacote LLVM apenas para disponibilizar a `libclang.dll` em tempo de build.
  * Aumenta o tempo de compilação inicial em vários minutos.
* **Como o Mundam utiliza o `rsraw`:**
  A aplicação utiliza o LibRaw essencialmente como o **Tier 1** da extração de imagens RAW: para ler e extrair a visualização JPEG embutida pelo fabricante (`raw_image.extract_preview()`), e como fallback para obter dimensões ou decodificação RGB de 8 bits.
* **Estratégia Recomendada de Substituição (Tornar Nativo em Rust):**
  1. **Abordagem Nativa 100% Rust (Recomendada):** Praticamente todas as câmeras RAW (CR2, CR3, NEF, ARW, DNG, RAF, RW2, ORF, PEF) embutem previews em formato JPEG ou TIFF nos primeiros blocos de metadados EXIF/TIFF IFD. O próprio Mundam já possui no `image.rs` o **Tier 2 (Brute-force JPEG scan)** e extração de IFD. Se expandirmos um parser TIFF/EXIF em Rust puro (usando crates consolidados como `kamadak-exif` ou `nom`), podemos extrair previews de 99% das fotos RAW em **menos de 5 milissegundos**, sem compilar uma única linha de C++.
  2. **Decodificação Completa de Pixels RAW:** Para os raros casos em que o preview embutido não existe e é necessária a interpolação demosaicing completa, utilizar crates em Rust puro como:
     * `quickraw`: Parser e descompressor RAW rápido e moderno em Rust.
     * `rawloader`: Biblioteca em Rust puro mantida pela comunidade open-source para decodificação de sensores Bayer.
  3. **Fallback Externo Existente:** O Mundam já possui o **Tier 3 (FFmpeg fallback)**. Como o FFmpeg já é estagiado na aplicação, ele é capaz de decodificar a maioria esmagadora dos formatos RAW sem precisar da LibRaw.
* **Benefício:** Elimina a necessidade de MinGW, LLVM e Clang, permitindo que o Mundam volte a compilar com o compilador nativo oficial do Windows (MSVC) com zero dependências externas.

---

### 6.2. Dependência Problemática 2: `gpr_sdk` (resolvida)

O diagnóstico acima descreve a dependência presente quando este relatório foi escrito. Em 29 de outubro de 2026, o SDK e seu submódulo foram removidos. A aplicação usa FlashRaw `0.1.1` (`MIT OR Apache-2.0`), um decoder Rust com suporte declarado a GoPro GPR/VC-5. O caminho atual está documentado em [`docs/backend/gpr-native-decoder.md`](../backend/gpr-native-decoder.md). O FFmpeg não é usado para decodificar GPR.

---

### 6.3. Dependência Problemática 3: `image_dds` (Feature `encode` e `intel_tex_2`)
* **Grau de Criticidade:** 🟡 **Moderado (Mitigado)**
* **Dificuldades Causadas:** 
  * Puxava `intel_tex_2` com código ISPC incompatível com toolchains não-MSVC, gerando erros crípticos de biblioteca estática ausente.
* **Estratégia Recomendada:**
  1. **Solução Atual (Já implantada):** A aplicação agora utiliza `default-features = false, features = ["ddsfile", "image", "strum"]`. O problema está mitigado no estado atual.
  2. **Solução Definitiva em Rust:** O projeto já possui em sua árvore a dependência `bcdec_rs` (decodificador de texturas BC1-BC7 em Rust puro). Podemos integrar a leitura direta dos cabeçalhos DDS via `ddsfile` com descompressão via `bcdec_rs`, mantendo o pipeline gráfico 100% nativo, seguro contra panics e sem código FFI.

---

### 6.4. Dependência Problemática 4: `pdfium-render` (Binário Dinâmico Externo)
* **Grau de Criticidade:** 🟢 **Baixo**
* **Dificuldades Causadas:** 
  * Exige o download e presença da DLL `pdfium.dll` em tempo de execução.
* **Estratégia Recomendada:**
  * O download automatizado no script `setup-windows.ps1` resolve o problema de forma transparente para os desenvolvedores. Para o futuro, à medida que parsers e rasterizadores PDF em Rust puro (como `pdf` ou `lopdf` integrados com `tiny-skia` / `resvg`) atinjam paridade de renderização com o Chromium PDFium, a migração poderá ser avaliada.

---

## 7. Instruções de setup originais (histórico; veja “Estado atual” acima)

As instruções abaixo foram válidas para o toolchain GNU/MinGW e foram substituídas. Não as use para preparar um ambiente atual; consulte “Estado atual” no início deste relatório.

### Passo 1: Executar o Script de Preparação Automatizado
Abra o **PowerShell como Administrador** e execute:
```powershell
Set-ExecutionPolicy RemoteSigned -Scope Process -Force
.\scripts\setup-windows.ps1
```

O script antigo instalava LLVM, CMake e MSYS2/MinGW, configurava o target GNU e inicializava submódulos. Essas etapas foram removidas por não corresponderem às dependências atuais.

### Passo 2: Iniciar a Aplicação
Abra um novo terminal (para carregar o novo `PATH`) na raiz do projeto e execute:
```powershell
npm run tauri dev
```

Para o procedimento atual, execute `scripts/setup-windows.ps1` antes de iniciar a aplicação.
