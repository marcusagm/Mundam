# Elleven-Library Features 

## 1. Ecossistema de Captura, Ingestão e Fontes de Dados

Esta etapa foca em como os ativos entram no sistema, garantindo que o fluxo de trabalho não seja interrompido, seja na web ou no desktop.

### 1.1 Captura via Extensão de Navegador (Web Clipper)

Transforma o navegador em uma ferramenta de coleta ativa de referências.

* [ ] **Métodos de Captura de Imagem:**
* [ ] **Arrastar e Soltar (Drag & Drop):** Captura instantânea ao arrastar imagens para zonas de captura.
* [ ] **Atalhos Rápidos:** Uso de `Alt + Clique Direito` para salvamento imediato.
* [ ] **Captura em Lote (Batch Save):** Analisador de página para download múltiplo com filtros de tamanho e formato.
* [ ] **Seleção de Elementos (Element Picker):** Ferramenta para capturar imagens específicas de uma página.


* [ ] **Captura de Tela (Screenshots):**
* [ ] **Área Visível:** Captura do que está na tela.
* [ ] **Área Selecionada:** Recorte manual de regiões da página.
* [ ] **Página Inteira (Full Page):** Scroll automático para capturar o comprimento total do site.


* [ ] **Captura de URLs e Referências:**
* [x] **Thumbnails Optimization:** Geração paralela e eficiente de miniaturas (Native/FFmpeg/Rayon).
* [ ] **Bookmarks Visuais:** Salva o link com miniatura gerada automaticamente.
* [ ] **Mapeamento de Metadados:** Captura automática do Título, URL de origem e metadados de plataformas como Pinterest, ArtStation, Behance e Dribbble.



### 1.2 Ingestão de Desktop e Sistema Operacional

Integração profunda com os arquivos locais e outros softwares de design.

* **Monitoramento e Importação:**
* [x] **Monitoramento em Tempo Real (Folder Watching/Auto-import):** Vigia diretórios específicos e importa novos arquivos automaticamente.
* [x] **Mapeamento de Estrutura:** Opção de replicar a hierarquia original de pastas durante a importação.
* [x] **Detecção de Alterações:** Sincronização automática para remoções e renomeações de arquivos no disco.


* [ ] **Integração via Clipboard (Área de Transferência):**
* [ ] **Colagem Direta:** Detecta imagens ou arquivos copiados e cria o ativo via `Ctrl/Cmd + V`.


* [ ] **Captura de Sistema:**
* [ ] **Atalhos Globais:** Captura de tela independente do navegador para registrar outros softwares (Photoshop, Figma, etc.).



### 1.3 Importação de Bibliotecas e Conteúdo Externo

Conectividade com serviços de terceiros e migração de dados.

* **Vincular Vídeos Online:** Adição de URLs do **YouTube** e **Vimeo** com reprodução via player incorporado.
* **Migração de Terceiros:** Importação direta de pastas e pins do **Pinterest**.
* **Pacotes de Ativos (.eaglepack):** Importação e exportação de coleções pré-categorizadas para compartilhamento.
* **Manutenção de Locais:** Recuperação de diretórios movidos ou renomeados e identificação de caminhos quebrados.

## 2. Organização, Inteligência e Gerenciamento de Metadados

Esta etapa detalha como o sistema transforma arquivos brutos em uma biblioteca estruturada e pesquisável.

### 2.1 Gerenciamento de Bibliotecas e Estrutura de Pastas

Permite a separação lógica e física de grandes volumes de ativos.

* [ ] **Modelo Multi-Biblioteca:** Suporte para criar e alternar rapidamente entre diferentes "containers" ou bases de dados independentes.
* [ ] **Consolidação (Merge):** Capacidade de unir duas ou mais bibliotecas em uma estrutura única.
* [ ] **Segurança de Acesso:** Opção para proteger pastas ou bibliotecas específicas com senha.
* [x] **Hierarquia de Pastas Clássica:**
* [x] **Pastas e Subpastas:** Suporte a múltiplos níveis de profundidade para categorias fixas.
* [x] **Visualização Recursiva:** Alternância entre visualização plana (apenas arquivos da pasta) e recursiva (incluindo subpastas).
* [ ] **Customização Visual:** Atribuição de cores e ícones para identificação rápida de pastas.
* [ ] **Agrupamento Virtual:** Recurso para agrupar pastas relacionadas sem alterar o caminho físico no disco.



### 2.2 Ecossistema de Etiquetas (Tags) e Taxonomia

Um sistema flexível que permite múltiplas camadas de classificação.

* [x] **Estrutura Hierárquica de Tags:** Criação de relações "pai e filho" para organizar conceitos (ex: `Arquitetura > Moderna > Fachada`).
* [x] **Visualização em Árvore:** Interface dedicada para navegar pela taxonomia com estados de expansão e colapso.
* [x] **Grupos de Tags:** Organização de etiquetas em grupos coloridos para facilitar a seleção em massa e a gestão visual.
* **Operações Avançadas de Taxonomia:**
* [ ] **Mesclagem (Tag Merge):** Consolidação de etiquetas duplicadas ou similares em uma única categoria.
* [ ] **Gestão Global:** Painel centralizado para renomear, mover entre ramos ou excluir etiquetas em toda a biblioteca.


* **Atribuição Inteligente:**
* [ ] **Sugestão e Autocompletar:** Sistema que sugere etiquetas baseadas no histórico de uso ou ativos similares durante a digitação.
* [ ] **Edição em Lote (Bulk Tagging):** Vinculação de etiquetas a múltiplos arquivos simultaneamente.


### 2.3 Pastas Inteligentes e Filtros Dinâmicos

Unidades de organização automatizada baseadas em regras lógicas.

* [x] **Lógica Booleana e Operadores:** Suporte a filtros complexos utilizando "E" (AND), "OU" (OR) e "NÃO" (NOT).
* [x] **Critérios de Automação:** Agrupamento automático de ativos que atendam a critérios como:
* [x] Nome do arquivo
* [x] extensões
* [ ] URLs de origem.
* [x] Presença ou ausência de tags específicas.
* [x] Intervalos de datas (criação, importação ou modificação).
* [ ] Dimensões, proporção (aspect ratio)
* [x] tamanho do arquivo.
* [ ] Cores predominantes
* [x] Avaliações (ratings).


* [x] **Atualização em Tempo Real (Auto-Update):** Novos ativos são incluídos na pasta inteligente assim que satisfazem as regras definidas.

### 2.4 Análise Cromática e Inspeção Visual

Processamento automatizado para busca estética.

* [ ] **Extração Automática de Cores:** Identificação das cores principais (5 a 10 tons) de cada imagem ou vídeo importado.
* [ ] **Busca por Proximidade:** Localização de ativos através de um seletor visual de cores ou inserção de códigos HEX.
* [ ] **Ajuste de Tolerância:** Controle de sensibilidade para expandir ou restringir a precisão da cor pesquisada.

### 2.5 Metadados e Propriedades Personalizadas

Expansão da informação técnica associada a cada ativo.

* [ ] **Integração Standard (EXIF/IPTC):** Leitura automática de dados de câmera, ISO, abertura, localização e data original.
* [ ] **Propriedades Personalizadas (Extra Properties):** Criação de campos de dados definidos pelo usuário (texto, seleção, etc.) para catalogação técnica detalhada.
* [ ] **Anotações e Comentários:**
* [ ] **Anotações em Área:** Capacidade de "desenhar" retângulos sobre regiões específicas da imagem para comentários focais.
* [x] **Notas de Ativo:** Campo de texto livre para descrições longas ou documentação técnica.
* [x] **Avaliação por Estrelas:** Sistema de rating de 1 a 5 para priorização de qualidade.

## 3. Visualização, Inspeção e Suporte a Formatos

Este módulo transforma a biblioteca em um "canivete suíço" visual, permitindo a análise profunda de ativos sem a necessidade de abrir softwares externos pesados.

### 3.1 Motor de Renderização e Visualização Universal

Capacidade de exibir e processar uma vasta gama de ativos com alta fidelidade.

* [x] **Suporte Multiformato Profissional:** Visualização de mais de 90 formatos, incluindo imagens tradicionais, vetores, vídeos, áudios e arquivos 3D.
* [x] **Renderização de Arquivos Proprietários:** Pré-visualização direta de arquivos do ecossistema de design como `.psd`, `.ai`, `.xd`, `.sketch`, `.affinity` e `.fig`.
* [x] **Formatos Web e Modernos:** Suporte nativo para `.webp`, `.avif`, `.heic`, `.svg`, além de formatos profissionais como `.exr`, `.hdr` e `.tiff`.
* [x] **Previews Persistentes:** Geração de miniaturas de alta fidelidade que permitem a visualização instantânea de arquivos pesados (ex: PSDs de 2GB) sem consumo excessivo de RAM.
* [x] **Carregamento Progressivo:** Uso de miniaturas para navegação rápida e carregamento de imagens em alta definição apenas sob demanda.

### 3.2 Experiência de Navegação e Layouts Dinâmicos

Diferentes modos de visualização para se adaptar ao fluxo de trabalho do usuário.

* [x] **Layout Masonry de Alta Performance:** Organização otimizada de imagens com proporções variadas, utilizando processamento via **Wasm/Rust** para garantir fluidez.
* [x] **Layouts de Grade e Lista:** Opções de visualização simétrica tradicional ou detalhada com metadados em colunas.
* [x] **Modo de Inspeção (Slide Mode):** Visualização em tela cheia com navegação por gestos ou teclado.
* [x] **Zoom Dinâmico e Pan:** Ferramentas para inspeção de detalhes em alta resolução e navegação dentro de arquivos grandes.

### 3.3 Inspeção Técnica de Vídeo, Áudio e Animação

Ferramentas de "baixo nível" para análise de mídia temporal.

* [ ] **Hover Preview (Scrubbing):** Pré-visualização do conteúdo de vídeos e GIFs apenas ao passar o mouse sobre a miniatura.
* [ ] **Anotações com Timestamp:** Capacidade de adicionar comentários em momentos específicos do vídeo ou áudio, funcionando como marcadores de tempo.
* [ ] **Controles de Reprodução Avançados:**
* [ ] **Loop de Trecho:** Definição de pontos "A" e "B" para repetição contínua.
* [ ] **Ajuste de Velocidade:** Reprodução de 0.5x a 2x para análise técnica.
* [ ] **Visualização Frame a Frame:** Navegação por quadros individuais em GIFs e vídeos.



### 3.4 Gerenciamento e Visualização de Fontes

Atua como um hub central para tipografia, eliminando softwares de terceiros.

* [ ] **Preview de Texto Customizado:** Permite digitar frases personalizadas e visualizá-las instantaneamente em todas as fontes da biblioteca.
* [ ] **Ativação em Um Clique:** Ativa ou desativa fontes no sistema operacional diretamente pela interface do aplicativo.
* [ ] **Filtros de Classificação Tipográfica:** Busca refinada por categorias como serifa, sem serifa, manuscrita ou monoespaçada.

### 3.5 Inspeção de Ativos 3D

Visualização e navegação em modelos tridimensionais sem engines de renderização.

* [ ] **Formatos Suportados:** Inspeção de arquivos `.obj`, `.fbx`, `.stl` e `.gltf`.
* [ ] **Navegação Orbital 360°:** Rotação completa, pan e zoom no modelo dentro do painel de visualização.
* [ ] **Modos de Visualização Técnica:** Alternância entre renderização texturizada, *wireframe* (aramado) e inspeção de malha.

### 3.6 Painel de Propriedades e Metadados Técnicos

Extração de dados profundos para conferência técnica.

* [x] **Inspetor EXIF/IPTC:** Visualização detalhada de dados de captura como ISO, abertura e localização geográfica.
* [ ] **Paleta de Cores Dinâmica:** Listagem de códigos HEX das cores predominantes com funcionalidade de cópia rápida.
* [x] **Estatísticas Físicas:** Exibição de dimensões exatas, DPI, tamanho em disco e datas de criação/modificação.


## 4. Busca, Recuperação e Processamento em Lote

Este módulo garante que a recuperação da informação seja instantânea e que tarefas repetitivas sejam automatizadas para máxima produtividade.

### 4.1 Motor de Busca Global e Inteligente

Algoritmos avançados para localização de ativos por contexto ou texto.

* [x] **Busca por Texto Completo (Fuzzy Search):** Pesquisa em nomes de arquivos, extensões, tags, notas e até URLs de origem.
* [ ] **Tolerância a Erros (Fuzzy Matching):** Algoritmo que encontra resultados aproximados mesmo com termos incompletos ou pequenos erros de digitação.
* [ ] **Busca por Fonte e Origem:** Localiza imagens através do link original do site onde foram capturadas ou caminhos relativos de armazenamento.
* [ ] **Busca por Paleta de Cores:** Filtro visual que permite localizar ativos através de tons específicos selecionados em um seletor.

### 4.2 Construtor de Pesquisa Avançada e Filtros Multidimensionais

Interface para consultas complexas baseadas em propriedades físicas e metadados técnicos.

* [ ] **Lógica Booleana Completa:** Suporte a operadores lógicos **E** (AND), **OU** (OR) e **NÃO** (NOT) para combinar critérios de busca.
* [ ] **Critérios de Refinamento Técnico:**
* [x] **Resolução e Proporção:** Filtros por dimensões exatas (largura/altura) ou proporções de tela (Retrato, Paisagem, Quadrado).
* [x] **Atributos de Arquivo:** Filtragem por extensão (formato), tamanho do arquivo e datas (criação, modificação ou importação).
* [x] **Qualidade e Status:** Busca por ativos com ou sem tags/anotações e por classificação (estrelas).
* [x] **Consultas Salvas (Saved Searches):** (Implementado via Pastas Inteligentes).

### 4.3 Processamento e Ações em Lote (Batch Processing)

Ferramentas para manipulação massiva de arquivos em uma única operação.

* [ ] **Edição em Massa:** Adição ou remoção de tags, alteração de ratings e movimentação entre pastas para múltiplos arquivos simultaneamente.
* [ ] **Renomeação Sequencial:** Ferramenta avançada para renomear centenas de arquivos seguindo padrões de numeração, prefixos, sufixos e substituição de strings.
* [ ] **Conversão de Formato em Lote:** Capacidade de converter grupos de imagens para formatos mais leves (como WebP ou JPG) definindo qualidade e redimensionamento.
* [ ] **Automação de Fluxo (Actions):** Criação de macros ou "receitas" (ex: "Tag X + Mover para Pasta Y") disparadas por atalhos de teclado customizáveis.

### 4.4 Higiene e Manutenção da Biblioteca

Recursos para garantir a integridade e limpeza do banco de dados.

* [ ] **Busca por Duplicatas:** Identificação de arquivos idênticos ou muito similares via hash para liberar espaço em disco.
* [ ] **Manutenção de Integridade de Links:**
* [ ] **Localizador de Caminhos Quebrados:** Identifica ativos cujos arquivos originais foram movidos ou deletados fora do sistema.
* [ ] **Recuperação de Locais (Location Recovery):** Ferramenta para remapear diretórios raiz que foram renomeados no sistema operacional.
* [ ] **Otimização de Banco de Dados:** Recurso para reconstruir índices e compactar a base de dados local para manter a performance em bibliotecas massivas.


## 5. Fluxo de Trabalho, Infraestrutura e Personalização

Este módulo final detalha como o sistema se integra ao sistema operacional, garante a segurança dos dados e permite que o usuário molde a ferramenta às suas necessidades.

### 5.1 Sincronização e Mobilidade de Dados

Modelo centrado na privacidade e na liberdade de escolha do provedor de nuvem.

* [ ] **Modelo Cloud-Agnostic:** O software não possui nuvem proprietária, mas integra-se a pastas locais do **Dropbox, Google Drive, OneDrive, iCloud e pCloud** para sincronização.
* [ ] **Sincronização Multi-dispositivo:** Permite acessar a mesma biblioteca em diferentes computadores através de pastas sincronizadas.
* [ ] **Controle de Conflitos:** Sistema de alerta que solicita o fechamento de instâncias em outros dispositivos para evitar corrupção de dados ao editar simultaneamente.
* [ ] **Acesso Mobile:** Versões para iOS e Android focadas em visualização e gerenciamento remoto via Wi-Fi.

### 5.2 Interoperabilidade e Integração Criativa

Conectividade direta com o ecossistema de softwares de design e edição.

* [x] **Drag & Drop Universal:** Suporte para arrastar ativos diretamente para o canvas de ferramentas como **Figma, Photoshop, Illustrator, After Effects, Premiere e Canva**.
* [ ] **Plugins Dedicados:** Extensões específicas (ex: Figma) para navegar na biblioteca sem sair da tela de design.
* [ ] **Cópia de Código HEX:** Integração com a área de transferência para colar paletas de cores extraídas diretamente em campos de preenchimento de ferramentas de UI.

### 5.3 Exportação e Portabilidade

Flexibilidade para retirar e compartilhar dados organizados.

* [ ] **Exportação com Metadados:** Salva arquivos em pastas locais preservando a estrutura hierárquica e incluindo informações de tags e notas.
* [ ] **Pacotes Proprietários (.eaglepack):** Compactação de seleções de ativos com todos os metadados para compartilhamento entre usuários do sistema.
* [ ] **Backup de Banco de Dados:** Ferramentas para exportar a base de dados e a taxonomia de etiquetas para transferência segura.

### 5.4 Personalização e Interface do Usuário

Ajuste da ferramenta ao gosto visual e à produtividade do usuário.

* [ ] **Gerenciamento de Aparência (Themes):** Suporte a temas **Escuro, Claro, Cinza e Monocromático**.
* [ ] **Personalização Avançada:** Opção para aplicação de **CSS customizado** por usuários avançados.
* [ ] **Gestão de Atalhos (Hotkeys):** Configuração total de teclas rápidas para navegação, classificação e disparar ações de automação.
* [x] **Settings Modal:** Painel de configurações unificado e modular.
* [x] **Status Bar Toggles:** Alternância rápida de painéis laterais.
* [ ] **Preferências de Inicialização:** Configuração do comportamento ao abrir (abrir última pesquisa, mostrar biblioteca vazia) e definição de formatos suportados.

### 5.5 Infraestrutura Técnica e Performance

Mecanismos de fundo que garantem a velocidade e a integridade dos arquivos.

* [ ] **Paradigma Local-First:** Toda a inteligência de busca e visualização é processada na máquina do usuário, utilizando arquivos JSON e estruturas de pastas locais.
* [ ] **Motor de Alta Performance (Rust/Wasm):** Uso de WebAssembly e Rust para processamento de layouts complexos e decodificação de formatos pesados com velocidade nativa.
* [x] **Geração Assíncrona de Miniaturas:** Worker dedicado em threads separadas para criar previews de arquivos profissionais (PSD, TIFF, HDR) sem travar a interface.
* [ ] **Agendamento de Backups:** Criação automática de cópias de segurança da base de dados com sistema de rotação para economizar disco.
* [x] **Arquitetura de Dados:** Otimização interna utilizando **SQLite ou TypeORM** para garantir consultas rápidas em bases massivas.

### 5.6 Diferenciais e Limitações Identificadas

Observações sobre o comportamento do sistema em escala.

* [ ] **Escalabilidade Extrema:** Capacidade comprovada de lidar com mais de **100.000 ativos** mantendo a fluidez de busca.
* [ ] **Fluxo Não-Destrutivo:** Os ativos originais permanecem intactos; todas as edições de metadados ocorrem em uma camada de software separada.
* [ ] **Dependência de Hardware:** O processamento inicial e a análise cromática de grandes volumes são intensivos em CPU e RAM.
* [ ] **Colaboração em Equipe:** Limitada ao compartilhamento de pastas de nuvem sincronizadas, sem suporte nativo a permissões de múltiplos usuários.
