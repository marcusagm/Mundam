# Implementação HLS Audio e Otimizações de Player
**Data:** 2026-02-09
**Status:** Concluído

## Visão Geral

Este documento detalha as modificações realizadas para implementar suporte robusto a streaming HLS para arquivos de áudio que requerem transcodificação (ex: `dts`, `wv`, `ape`, `aifc`, `amr`), bem como melhorias na experiência do usuário (UX) relacionadas a loading states e controle de playback nos componentes `AudioInspector` e `AudioRenderer`.

## Objetivos Alcançados

1.  **Suporte Ampliado de Formatos de Áudio**: Implementação de pipeline HLS para formatos não nativos que causavam travamento da UI ou falha de reprodução.
2.  **Correção de Loading Infinito**: Resolução de spinners travados no Inspector e Renderer devido a incompatibilidades de eventos (Safari/macOS) e timeouts de transcodificação.
3.  **Melhoria de UX/UI**: Separação visual de carregamento de mídia vs. carregamento de waveform, e garantia de controle do player.

## Detalhamento Técnico das Modificações

### 1. Backend (Rust)

#### Detecção e Classificação (`src-tauri/src/transcoding/detector.rs`)
-   **Mudança**: Adição dos formatos `dts`, `wv`, `ape`, `amr`, `aifc` à lista `TRANSCODE_AUDIO`.
-   **Motivo**: Esses formatos não são suportados nativamente pelo WebView (Safari) e exigem transcodificação via FFmpeg. Anteriormente, eram tratados incorretamente ou falhavam silenciosamente.

#### Transcodificação HLS (`src-tauri/src/streaming/segment.rs`)
-   **Lógica de Áudio-Only**: Implementada lógica específica para arquivos identificados como áudio.
    -   Argumentos FFmpeg: `-vn` (ignorar vídeo), `-c:a aac` (transcodificar para AAC).
    -   Mapeamento Seguro: Alterado mapeamento de streams para `-map 0:a:0?`. O sufixo `?` torna o mapeamento opcional, prevenindo erros do tipo "Stream map matches no streams" quando o FFmpeg não encontra o stream esperado (comum em arquivos `monkey's audio` ou `dts` raw).

### 2. Frontend (TypeScript/SolidJS)

#### Audio Player Core (`src/components/ui/AudioPlayer.tsx`)
-   **Reatividade de Referência**: Conversão da `audioRef` para um SolidJS Signal. Isso corrigiu um bug crítico onde o hook de HLS inicializava antes da referência do elemento Audio estar montada.
-   **Correção de Carregamento (macOS/Safari)**:
    -   Alterado atributo `preload` de `"metadata"` para `"auto"`.
    -   **Contexto**: O Safari nativo (usado para HLS no macOS) não dispara o evento `loadeddata` (necessário para limpar o estado de loading) se `preload="metadata"` estiver definido e não houver `autoplay`. Isso causava o "spinner infinito" no Inspector.
-   **Timout de Waveform**: Introduzido um timeout de 15 segundos para a geração da waveform.
    -   **Motivo**: Arquivos complexos como `ape` demoram muito para transcodificar. Se o backend demorar, o player não deve ficar travado para sempre. Agora, após 15s, a waveform falha silenciosamente (exibindo linha vazia) mas o áudio toca.
-   **Lógica de Loader Estrita**:
    -   O loader principal agora bloqueia a interface até que `isActuallyLoading` seja falso.
    -   `isActuallyLoading` considera tanto o carregamento do HLS (`loadeddata`) quanto a waveform.
    -   **Comportamento**: Para arquivos rápidos, é instantâneo. Para `ape`, o loader persiste por até 15s (timeout da waveform) ou menos se terminar antes, garantindo que o usuário não tente tocar antes de "tudo estar pronto", conforme solicitado.

#### Estilos e UX (`src/components/ui/audio-player.css`)
-   **Overlay de Loader**: Configurado para bloquear interações (`pointer-events` ativo) enquanto visível, forçando o usuário a esperar o carregamento completo.
-   **Remoção de Spinners Secundários**: Removido o spinner específico da área de waveform para limpar a interface, centralizando o feedback no loader principal.

#### Componentes Consumidores
-   **AudioInspector (`src/components/features/inspector/audio/AudioInspector.tsx`)**:
    -   Removida lógica de loader artificial (timeout de 400ms) que causava sobreposição e conflito visual. Agora confia puramente no estado do `AudioPlayer`.
-   **AudioRenderer (`src/components/features/itemview/renderers/audio/AudioRenderer.tsx`)**:
    -   Removido overlay de carregamento manual. Confia no `AudioPlayer` para feedback visual, garantindo consistência entre visualização de item e inspector.

#### Utils (`src/lib/stream-utils.ts`)
-   Atualização das listas de extensão para garantir que formatos exóticos sejam roteados corretamente para a pipeline de áudio (e não vídeo ou genérico).

## Resultado Final

-   **Formatos Suportados**: `mp3`, `wav`, `flac`, `ogg`, `oga`, `opus`, `wma`, `ac3`, `dts`, `wv`, `ape`, `aifc`, `amr`.
-   **Experiência**:
    -   Arquivos leves tocam quase instantaneamente.
    -   Arquivos pesados (`ape`) mostram loader por tempo necessário (max 15s) e então liberam o playback seguro.
    -   Interface consistente entre Inspector (compacto) e Renderer (full).

## Próximos Passos Sugeridos

1.  Investigar otimização de performance do FFmpeg para decodificação de `ape` e `dts` (talvez pré-transcodificação em cache).
2.  Implementar visualização de erro mais detalhada caso o timeout de 15s seja atingido (atualmente falha silenciosamente para permitir playback).
