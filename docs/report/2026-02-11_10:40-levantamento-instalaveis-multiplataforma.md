# Levantamento — Preparar instaláveis Windows, macOS e Linux (FFmpeg + Assimp)

## Objetivo
Mapear o que precisa ser alterado para que o build de release gere instaláveis confiáveis para **Windows, macOS e Linux**, com atenção especial ao empacotamento de dependências externas (`ffmpeg`, `ffprobe`, `assimp`).

---

## 1) Diagnóstico do estado atual

### 1.1 Empacotamento no Tauri
Hoje o `tauri.conf.json` está copiando `ffmpeg/*` e `assimp/*` como `resources` para o bundle final. Isso é simples, mas traz três problemas: tamanho do pacote, risco de incluir arquivos irrelevantes e baixa previsibilidade por plataforma. Além disso, o wildcard atual não deixa explícito quais binários entram por SO.  

### 1.2 FFmpeg: binário atual não é multiplataforma
O projeto possui **um único** `src-tauri/ffmpeg/ffmpeg` (~77 MB). A inspeção do header mostra assinatura Mach-O (`0xCF FA ED FE`), ou seja, binário de ecossistema Apple, não Linux/Windows.  

Consequência prática: builds Linux/Windows não terão um `ffmpeg` válido via recurso local, caindo em `PATH` (quando existir), o que torna o instalável não determinístico.

### 1.3 FFprobe não está sendo empacotado explicitamente
A aplicação usa `ffprobe` para metadados e tenta inferir o executável “ao lado” do `ffmpeg`. Se `ffprobe` não existir no pacote, o código tenta `ffprobe` no sistema. Isso quebra a premissa de instalável self-contained.

### 1.4 Assimp: árvore atual mistura artefatos e path de runtime inconsistente
A pasta `src-tauri/assimp` tem ~201 MB e mistura artefatos de múltiplos SOs (DLLs, SOs, instalador `.exe`, includes, docs, PDBs). O código atual procura caminhos que **não batem** com o layout existente:

- macOS: código procura `assimp/macos/assimp`, mas no repositório está em `assimp/macos/6.0.4/bin/assimp`.
- Windows: código procura `assimp/windows-x64/assimp.exe`, mas existe `assimp-vc143-mt.dll` (sem `assimp.exe`).
- Linux: código procura `assimp/linux/assimp`, mas há apenas `libassimp.so*` (sem CLI `assimp`).

Resultado: o fallback para `assimp` no `PATH` tende a ser acionado na maioria dos cenários, novamente não determinístico.

### 1.5 Estratégia de runtime depende demais de fallback em PATH
Tanto FFmpeg quanto Assimp têm lógica “best effort” para usar dependência do sistema quando o bundle não tem o executável correto. Para distribuição, isso reduz confiabilidade (máquina do usuário pode não ter nada instalado).

---

## 2) O que precisa mudar (mínimo viável para instaláveis confiáveis)

## 2.1 Separar assets por plataforma + arquitetura
Criar estrutura explícita, por exemplo:

- `src-tauri/vendor/ffmpeg/<target-triple>/{ffmpeg,ffprobe}`
- `src-tauri/vendor/assimp/<target-triple>/...`

Exemplos de target: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`.

## 2.2 Parar de versionar binários pesados “soltos” no repositório principal
Mover para uma estratégia reproduzível:

- script de provisionamento (`scripts/fetch-third-party.{sh,ps1}`),
- artefatos hospedados em release privada/pública (GitHub Releases, S3, etc.),
- validação por SHA-256 e licença.

Benefícios: repo mais leve, revisão melhor, trilha de compliance clara.

## 2.3 Ajustar estratégia de bundle no Tauri
Substituir wildcard amplo por declaração explícita e/ou `externalBin` para executáveis.

- Para `ffmpeg/ffprobe` (executáveis): priorizar configuração e naming que o Tauri reconheça por target.
- Para libs dinâmicas (`libassimp.so`, `.dylib`, `.dll`): decidir entre:
  - (A) distribuir apenas CLI `assimp` por SO (mais simples para o fluxo atual), ou
  - (B) abandonar CLI e usar binding/crate Rust (mais complexo, mas elimina subprocesso).

## 2.4 Normalizar resolução de paths no runtime
Centralizar descoberta de binários em um módulo único de “toolchain runtime”, com ordem clara:

1. binário embutido no bundle,
2. binário de desenvolvimento local (`vendor/...`),
3. fallback opcional em `PATH` (somente dev, com aviso).

Também incluir telemetria/log claro indicando **qual caminho foi escolhido**.

## 2.5 Empacotar `ffprobe` junto com `ffmpeg`
Garantir par completo em todos os targets, evitando fallback para sistema.

## 2.6 Definir matriz de build e validação por SO
Padronizar pipeline de build para cada SO/arch com checklist:

- build release,
- smoke test de inicialização do app,
- comando de sanidade: `ffmpeg -version`, `ffprobe -version`, `assimp version` (ou comando equivalente),
- teste funcional mínimo (gerar thumbnail de vídeo e converter um modelo para GLB).

---

## 3) Recomendação específica para FFmpeg e Assimp

### FFmpeg (recomendação forte)
1. Tratar `ffmpeg` + `ffprobe` como sidecars/binários versionados por target.
2. Parar de usar binário único na raiz `src-tauri/ffmpeg/ffmpeg`.
3. Implementar script de download/verificação por hash no CI e local.

### Assimp (recomendação pragmática em 2 fases)

**Fase 1 (rápida para release):**
- distribuir CLI `assimp` funcional por target (não DLL/SO avulsos sem executável),
- corrigir paths do código para layout real,
- remover payload de docs/headers/PDB do pacote final.

**Fase 2 (evolução):**
- avaliar migração de subprocesso `assimp export` para abordagem Rust nativa (binding FFI ou crate consolidada),
- objetivo: reduzir dependência operacional de binário externo.

---

## 4) Backlog proposto (ordem de execução)

1. **Inventário e decisão de distribuição** de FFmpeg/Assimp por target (fonte oficial, licença, hash).  
2. **Criar scripts de provisionamento** para baixar e preparar pasta `vendor/` por target.  
3. **Refatorar `tauri.conf.json`** para empacotamento explícito por plataforma (evitar wildcard amplo).  
4. **Refatorar `get_ffmpeg_path` e `get_assimp_path`** para resolver layout novo e logar origem.  
5. **Incluir `ffprobe` no pacote** e validar probe em runtime.  
6. **Reduzir payload do assimp** para apenas o necessário por target (sem headers/docs/pdb no bundle).  
7. **Adicionar pipeline CI/CD de release** com matriz Windows/macOS/Linux e smoke tests.  
8. **Executar teste de ponta a ponta** gerando instaláveis e validando em máquinas limpas.

---

## 5) Critérios de pronto

- Instalável de cada SO funciona em máquina limpa sem FFmpeg/Assimp pré-instalados.
- Logs de runtime informam caminhos efetivos de `ffmpeg`, `ffprobe` e `assimp`.
- Pipeline de release bloqueia publicação se smoke tests de dependências falharem.
- Tamanho de pacote reduzido/removido de artefatos desnecessários (docs, headers, símbolos de debug não usados).

---

## 6) Riscos e observações

- **Licenciamento**: validar redistribuição dos builds escolhidos de FFmpeg (LGPL/GPL dependendo de opções de build) e Assimp.
- **macOS notarization**: binários externos precisam assinatura adequada para distribuição fora de dev.
- **Arquiteturas**: ideal suportar `x64` e `arm64` em macOS e, se aplicável, Windows/Linux.

