# Roadmap Consolidado de Pendências e Novos Recursos (Mundam)

**Data:** 04 de Março de 2026
**Objetivo:** Consolidar todas as pendências ativas documentadas nos relatórios de análise (19 a 20 de Fevereiro), priorizando melhorias contínuas, manutenibilidade, resolução de bugs e, por fim, listando todos os novos recursos pendentes para a plataforma.

---

## 1. Arquitetura, Qualidade de Código e Backend

### 1.1 Testes e Qualidade
- [ ] **Testes Efetivos no Frontend**: Mitigar a ausência momentânea de testes de unidade criando testes focados em UI (com `@solidjs/testing-library`), compensando a flag base de build pass.
- [ ] **Testes Unitários para `LifecycleRegistry`**: 
  - [ ] Registrar e cancelar tasks, validando abort forçado.
  - [ ] Tratamento de nomes duplicados, forçando o isolamento.
  - [ ] Regras de árvore entre tokens "pai e filho".
  - [ ] Função matriz `shutdown_all`.

### 1.2 Refatorações de Banco e Backend
- [ ] **Estruturação de DTOs nas Consultas Rust**: Centralizar a tipagem de resposta na rotina de `rename_image` do banco (substituir o retorno gigante por uma Struct com suporte global a `FromRow`), evitando a flag de type complexity.

---

## 2. Sistema de Arquivos e Monitoramento (Sync Local)

### 2.1 Preservação de Dados
- [ ] **Renomeação e Deslocamento Offline**: Tratar o cenário onde o usuário renomeia pastas monitoradas com a aplicação completamente fechada, de maneira a não deflagrar uma deleção profunda mascarando a reindexação com perda total das tags cadastradas.
- [ ] **Renomeação Raiz Segura**: Bloquear a perda da base de configuração e metadata se a root inicial for alterada por conta de path.

### 2.2 Controle de Estrutura Nativo
- [ ] **Manuseio Integral da Árvore de Pastas**: Fornecer meios para que todo o fluxo de criar, mover, mesclar, deletar e renomear os arquivos em disco dentro da Root da aplicação seja feito atritamente sem o risco da perda da catalogação no SQLite.

---

## 3. UI/UX: Experiência da Viewport e Comportamentos Core

### 3.1 Otimização Visual do Viewport
- [ ] **Priorização Sensível na Geração de Thumbnails**: Refinar o pool para criar prioritariamente os assets interceptados visualmente pela tela no exato momento, baseados apenas na ordenação visual filtrada do usuário.
- [ ] **Seleção em Lote Intuitiva**: Adicionar seleção massiva "clique-e-arraste" (marquee view) a partir de áreas vazias dentro das visualizações em Grid e Masonry.
- [ ] **Persistência Absoluta**: Serializar o `ViewportPreferencesState` constantemente na tabela de configuração SQLite local assegurando permanência de visualização entre sessões.

### 3.2 Otimização de Permutações e Atalhos
- [ ] **Atalhos do Teclado no Viewport**: Adicionar mapeamento dinâmico unindo atalhos corretos na tela de visualização central.
- [ ] **Tab Global**: Prover via `Tab` a omissão responsiva de abas laterais (`LibrarySidebar` e `FileSidebar`) simulando "Modo Zen" similar aos produtos Adobe.
- [ ] **Reproveitamento de Elementos UI**: Incorporar lógicas padrão em forms no `SortConfiguration` reaproveitando os ToggleGroups estáticos em prol da economia de CSS/Componentes.
- [ ] **Debouncing Slider / Lazy Load Modais**: Controlar a sensibilidade dos redimensionadores da tela atrasando o reflow do Engine; E atrasar por Chunk dinâmico os modais atrelados aos filtros do Toolbar.

---

## 4. Visualização e Inspeção (ItemView & Player)

### 4.1 Janela de Mídia Principal (ItemView)
- [ ] Remodelar `ItemView` adicionando Header nome do arquivo acima dos comandos da Toolbar.
- [x] Acoplar função base para limitar o tamanho percentual superdimensionado na rolagem de Zoom.
- [x] Loader visual responsivo: Otimizar para evitar travamentos durante parse de arquivos pesados, abrindo chance do usuário desistir/fechar caso sinta lentidão excessiva e abortando processamentos não completos.
- [ ] Garantir painel que traga os datalhes completos de mídia, como submodal direto da tela Item.

### 4.2 Reprodutor Multi-Formato (Video HLS)
- [x] Restablecer e fixar a funcionalidade falha de `Fullscreen` dedicada ao VideoPlayer.
- [ ] Acoplar um reparo final que unifique a amostragem de tempo e scrubing `timetrack` nos buffers entregues em HLS Linear.
- [ ] Adicionar mecanismo de "Picture-In-Picture" (PIP) e controles contínuos de playback loopback nos menus do video.

### 4.3 Inspector Global
- [ ] Reter e priorizar em sessões persistentes quais guias do Accordeon lateral (como Metadata general info, Tags) foram deixadas minimizadas ou abertas, permitindo mais de uma ficar aberta.

---

## 5. Melhorias Gerais, Design e Sistema

### 5.1 Refinamentos Estéticos do App
- [ ] Resgatar padronização mais elegante e sem ambiguidades aos recortes de "Focus" em assets.
- [ ] Limpeza arquitetônica e visual dos estilos legados não integrados harmoniosamente.
- [ ] Atualizar o modal `Welcome Screen` para ser visualmente envolvente e simples na passagem inicial do set de Path Raiz do aplicativo.
- [ ] Criar um tour inicial para apresentar as funcionalidades do aplicativo.
- [ ] Criar um sistema de feedback para que os usuários possam reportar bugs e sugestões.
- [ ] Criar uma area para que o usuário possa contribuir financeiramente com o projeto.

### 5.2 Estrutura do Software
- [ ] Finalizar setup de diretrizes oficiais para branch e canais de releases da solução final compilada, resolvendo problemas de inclusão de dependências de ferramentas externas como Assimp, FFmpeg, etc.
- [ ] Emparelhar no backend do Tauri e pipeline os gatilhos para auto-update funcional a clientes da ponta.

---

## 6. Recursos Novos (State-of-The-Art & IA Avançada)
*(Novas funcionalidades documentadas orientadas ao produto premium DAM)*

### 6.1 Expansão do Motor de Consultas Semânticas
- [ ] **Busca Estrutural por Harmonia (Cores)**: Escalar as funções originais do indexador CIE-LAB recém implementado afim de aceitar proximidades entre blocos de coesão visual, não somente busca isolada decimal.
- [ ] **Busca Recursiva de Taxonomia**: Dar suporte interno ou UI afim da busca pai/filho rastrear os galhos inteiros de "Master Tags".

### 6.2 Governança Inteligente de Metadados
- [ ] **Workflows Colaborativos**: Disparar um modelo estrutural que acate comentários nativos, transições de status da edição visual (aprovação), e trilhas auditáveis de versões de mídias.
- [ ] **Inteligência de Tags Automatizadas**: Construir orquestrador assistente baseado em classificadores IA leves conectando recomendações sugeridas contextuais nos ativos sem preenchimento.
- [ ] **Qualidade de Acervos**: Exibir estatísticas vitais e "Scores de completude ou consistência" das coleções incentivando boa higenização da biblioteca.

### 6.3 Integrações Periféricas Híbridas (Sistemas Operacionais)
- [ ] **Cloud e Plugins Livres**: Permitir hooks nativos via app de scripting voltado à integração base com Google Drive/Dropbox e extensões limitadas de rotina em APIs JS de clientes de fora do app.
- [ ] **Empacotamento Universais (.mundampack)**: Mapear zip exportavel sem dependência do ambiente pai para fazer back-ups e pontes das indexações que agrupam Imagens + Arquivo estrutural de banco.
- [ ] **Injetor Clipboard Web**: Acatar dados importados por um Endpoint isolado HTTP Web capaz de ser acessível nos Chrome Extensions, e captador raw do `Ctrl+V` nativo universal com imagens da internet / desktop direto no aplicativo.

### 6.4 Apresentação Global e Liberdade Total
- [ ] **Engine Livre OKLCH (Temas)**: Adicionar camada totalitária em tokens base que aceite temas abertos ou personalização pura de paletas para substituir o switch binário Claro/Escuro local.
- [ ] **i18n Multi-idiomas**: Alinhar todo front com pacotes internacionais, expandindo ao inglês estrutural.
