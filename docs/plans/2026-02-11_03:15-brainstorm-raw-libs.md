## 🧠 Brainstorm: Otimização de Thumbnails RAW (Alternativas ao `rsraw`)

### Contexto
O uso atual do `rsraw` (LibRaw) está se mostrando instável ou lento para extração de thumbnails em certos arquivos RAW (CR2, ARW, NEF), com tempos de 10s-100s, sugerindo processamento excessivo (parsing total). O objetivo é encontrar uma biblioteca Rust pura ou estratégia que extraia **apenas** o JPEG embutido (preview) de forma instantânea (< 500ms), sem decodificar os dados brutos do sensor.

---

### Opção A: `rawloader` (Pure Rust)
Biblioteca Rust pura focada em *decodificação* de arquivos RAW (demosaicing).
[Link: crates.io/crates/rawloader](https://crates.io/crates/rawloader)

✅ **Pros:**
- **Pure Rust:** Sem dependências de C/C++ (LibRaw), compilação mais fácil e segura.
- **Safety:** Evita os riscos de segurança de decodificadores C antigos.

❌ **Cons:**
- **Foco em Decodificação:** Projetada para converter RAW -> RGB de alta qualidade. Tende a ser *mais lenta* que LibRaw para decodificação completa.
- **Extração de Thumbnail:** Não possui uma API otimizada explícita para "apenas extrair o thumbnail JPEG" sem ler o resto do arquivo. Pode sofrer do mesmo problema de performance (ler o arquivo todo).
- **Menor Suporte:** Cobre menos câmeras exóticas que o LibRaw (padrão da indústria).

📊 **Esforço:** Médio (Troca de API)

---

### Opção B: `kamadak-exif` ou `nom-exif` (Metadata Extraction)
Usar um parser de metadados leve para localizar apenas a tag EXIF `ThumbnailOffset` e `ThumbnailLength` ou as SubIFDs de preview (JPEGInterchangeFormat).
[Link: crates.io/crates/kamadak-exif](https://crates.io/crates/kamadak-exif)

✅ **Pros:**
- **Performance Extrema:** Lê apenas os cabeçalhos (KB) em vez do arquivo todo (MB). Complexidade O(1) vs O(N).
- **Foco Correto:** Resolve exatamente o problema: "Onde está o JPEG embutido?", ignorando o sensor data.
- **Pure Rust:** Implementações leves disponíveis.

❌ **Cons:**
- **Complexidade de Parsing:** RAWs (especialmente RAWs proprietários como CR3/ARW) escondem previews em locais não-padrão (Makernotes ou SubIFDs específicos) que parsers genéricos de EXIF podem não encontrar.
- **Manutenção:** Requer lógica customizada para cada fabricante se o padrão TIFF/EXIF não for seguido à risca.

📊 **Esforço:** Médio-Alto (Requer lógica de fallback robusta)

---

### Opção C: `preview_image_extractor` (Crate Especializada)
Existem crates focadas apenas nisso, como `raw-thumbnail` (se existir/estiver ativa) ou implementação manual de um parser TIFF mínimo.
A maioria dos arquivos RAW (CR2, NEF, ARW, DNG, PEF) são baseados em TIFF. Podemos usar a crate `tiff` ou um parser manual para navegar nas IFDs e encontrar a que contém o JPEG.

✅ **Pros:**
- **Equilíbrio Ideal:** Mais robusto que apenas EXIF, mas muito mais leve que um decodificador RAW completo.
- **Controle:** Podemos implementar lógica específica para encontrar o "maior preview disponível" sem decodificar nada.

❌ **Cons:**
- **Implementação Manual:** Pode exigir escrever um parser TIFF simplificado para navegar nas tags proprietárias se nenhuma crate pronta atender.

📊 **Esforço:** Alto (Implementação de parser)

---

## 💡 Recomendação

**Opção Inicial: Testar `rawloader` conforme solicitado.**
Você sugeriu testar o `rawloader`. Vamos tentar implementá-lo rapidamente para ver se ele oferece uma extração de thumbnail rápida "out of the box".

**Opção Secundária (Se `rawloader` for lento): Abordagem Híbrida (TIFF Parsing)**
Se o `rawloader` também for lento (provável, pois é um decoder), a solução definitiva será implementar um **Extrator TIFF Leve**. A maioria dos arquivos RAW que estão lentos (CR2, NEF, ARW) são TIFFs válidos. Podemos usar a crate `tiff` para apenas ler as tags de diretório e extrair o blob JPEG, ignorando o resto.

### Próximo Passo
Vamos implementar a **Opção A (`rawloader`)** agora, conforme seu pedido, para validar a performance.

**Plano de Ação:**
1. Adicionar `rawloader` e `image` (se necessário).
2. Criar `src-tauri/src/thumbnails/raw.rs` usando `rawloader`.
3. Medir o tempo de extração.
