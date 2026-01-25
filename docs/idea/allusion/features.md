O Allusion é uma ferramenta de gerenciamento e organização de referências visuais (imagens) projetada para artistas e designers, funcionando como uma biblioteca centralizada com recursos avançados de catalogação e busca.

---

### 1. Gerenciamento de Locais e Bibliotecas (Locations)

Permite ao usuário definir, monitorar e organizar as origens físicas dos arquivos de imagem no sistema de arquivos local.

* **Adição de Locais (Root Folders)**
* Seleção de diretórios via diálogo do sistema.
* Suporte a Drag-and-Drop para importação de pastas.


* **Monitoramento em Tempo Real (Folder Watching)**
* Sincronização automática de novos arquivos adicionados às pastas monitoradas.
* Detecção de remoções e renomeações de arquivos no disco.


* **Manutenção de Integridade**
* Recuperação de locais (Location Recovery) para diretórios movidos ou renomeados.
* Identificação de caminhos quebrados ou inacessíveis.


* **Organização de Fontes**
* Criação de hierarquia de pastas dentro do aplicativo.
* Capacidade de mover e organizar locais de armazenamento via interface.



### 2. Organização e Gerenciamento de Etiquetas (Tags)

Sistema central de classificação taxonômica para imagens, utilizando uma estrutura hierárquica para facilitar a catalogação de grandes volumes de referências.

* **Estrutura Hierárquica de Tags**
* Criação de tags pai e tags filhas (árvore de tags).
* Visualização em árvore (Tags Tree) com estados de expansão e colapso.


* **Operações de Edição de Taxonomia**
* Renomeação de etiquetas.
* Atribuição de cores personalizadas a etiquetas específicas.
* Mesclagem de tags (Tag Merge) para consolidar categorias duplicadas.
* Movimentação de tags entre diferentes ramos da hierarquia.


* **Atribuição e Desatribuição**
* Vinculação de etiquetas a arquivos individuais ou múltiplos arquivos simultaneamente.
* Interface de seleção rápida de tags com sugestões automáticas (Auto-complete).


* **Busca de Taxonomia**
* Filtragem interna na lista de tags para localização rápida de etiquetas criadas.



### 3. Visualização e Navegação de Mídia (Content View)

Motor de renderização e interface de exploração visual das imagens catalogadas na biblioteca.

* **Layouts de Exibição Dinâmicos**
* **Layout Masonry:** Organização otimizada de imagens com diferentes proporções (implementado via motor em Rust/Wasm para alta performance).
* **Layout de Lista:** Visualização detalhada com metadados em colunas.
* **Layout de Grade (Grid):** Visualização simétrica tradicional.


* **Modo de Inspeção (Slide Mode / Zoom Pan)**
* Visualização em tela cheia de arquivos individuais.
* Navegação por gestos ou teclado (anterior/próxima).
* Ferramenta de Zoom dinâmico e Pan (arrastar) para inspeção de detalhes em alta resolução.


* **Carregamento Progressivo**
* Renderização de miniaturas (thumbnails) para navegação rápida.
* Carregamento de imagens em alta definição apenas sob demanda para economia de recursos.


* **Ações de Arquivo**
* Abertura de arquivos no visualizador padrão do sistema operacional.
* Revelação do arquivo no gerenciador de arquivos (Explorer/Finder).
* Cópia do arquivo para o clipboard.



### 4. Sistema de Pesquisa e Filtragem (Search System)

Mecanismos para localização de arquivos baseados em múltiplos critérios, desde buscas simples até consultas complexas.

* **Busca Rápida**
* Barra de pesquisa global para filtragem por nome de arquivo ou tags básicas.


* **Construtor de Pesquisa Avançada (Criteria Builder)**
* Criação de filtros lógicos baseados em:
* Presença ou ausência de tags específicas.
* Tipo de arquivo (extensão).
* Datas de criação ou modificação.
* Dimensões da imagem (largura/altura).
* Tamanho do arquivo.


* Suporte a operadores booleanos (E, OU, NÃO) para combinar critérios.


* **Consultas Salvas (Saved Searches)**
* Persistência de critérios de busca complexos para acesso rápido posterior.
* Atualização dinâmica: a busca salva reflete novos arquivos que atendam aos critérios automaticamente.

### 5. Gestão de Metadados e Propriedades Extra

Capacidade de expandir a informação associada a cada imagem para além das etiquetas, permitindo uma catalogação técnica detalhada.

* **Integração com Metadados Standard (EXIF/IPTC)**
* Leitura automática de dados técnicos da imagem (câmara, configurações de exposição, data original).
* Escrita de etiquetas Allusion diretamente nos metadados do ficheiro (opcional, via ExifTool).


* **Propriedades Personalizadas (Extra Properties)**
* Criação de campos de dados definidos pelo utilizador.
* Suporte a diferentes tipos de entrada (Texto, Seleção, etc.).
* Edição individual ou em lote (Bulk Edit) de propriedades adicionais.


* **Mapeamento de Fonte e Origem**
* Registo automático do URL de origem para imagens capturadas da web.
* Armazenamento de caminhos relativos e absolutos para manter a portabilidade da biblioteca.



### 6. Sistema de Captura Web (Allusion Clipper)

Funcionalidade de integração com navegadores para permitir a importação direta de referências visuais sem sair da web.

* **Extensão de Browser Dedicada**
* Ferramenta de seleção de elementos (Element Picker) para capturar imagens específicas de uma página.
* Captura de metadados da página de origem (Título, URL).


* **Servidor de Receção Local (Clipper Server)**
* Serviço interno que escuta pedidos da extensão e descarrega os ficheiros diretamente para as pastas monitorizadas.


* **Atribuição Rápida no Momento da Captura**
* Interface para adicionar etiquetas ou notas antes de confirmar a importação para a biblioteca.



### 7. Configurações e Personalização do Sistema

Painel central para ajustar o comportamento do Allusion às preferências de fluxo de trabalho do utilizador.

* **Personalização Visual (Appearance)**
* Suporte a Temas (Dark, Light, Dimmed, Monochrome).
* Opção de aplicação de CSS personalizado para utilizadores avançados.


* **Gestão de Atalhos de Teclado (Hotkeys)**
* Configuração de teclas de atalho para quase todas as ações do sistema (Navegação, Etiquetas, Pesquisa).
* Suporte a combinações complexas para produtividade acelerada.


* **Preferências de Startup e Uso**
* Definição do comportamento ao iniciar (abrir última pesquisa, mostrar biblioteca vazia, etc.).
* Configuração de formatos de imagem suportados e ignorados.


* **Ferramentas de Importação e Exportação**
* Exportação da base de dados e taxonomia de etiquetas para backup ou transferência.
* Importação de dados de outras bibliotecas Allusion.



### 8. Processos de Background e Infraestrutura Técnica

Mecanismos automáticos que garantem a performance e a segurança dos dados sem interrupção da experiência do utilizador.

* **Agendamento de Backups (Backup Scheduler)**
* Criação automática de cópias de segurança da base de dados em intervalos definidos.
* Rotação de backups para evitar consumo excessivo de disco.


* **Motor de Performance Wasm/Rust**
* Processamento de layouts complexos (Masonry) e decodificação de formatos pesados (como EXR) utilizando WebAssembly para velocidade nativa.


* **Geração de Miniaturas (Thumbnail Generation Worker)**
* Processamento assíncrono em threads separadas para criar miniaturas sem bloquear a interface.
* Suporte a formatos profissionais (PSD, TIFF, HDR/EXR) via leitores especializados.


* **Limpeza e Manutenção de Dados**
* Ferramentas para remover entradas de ficheiros que já não existem em disco.
* Otimização da base de dados local (SQLite/TypeORM).
