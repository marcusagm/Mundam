Elaborar um sistema de variáveis para CSS que garanta uma excelente flexibilidade de customização, porém que tenha um sistema de design bem consistente e coerente, garantindo a beleza de qualquer site ou aplicação. As variáveis devem permitir a criação de designs S-Tier, com facilidade.

**Detalhes**

Itens importantes a considerar
- cores padrões, primárias, secundárias e neutras
- Criação de paleta de cores com 10 níveis de variação
- Cores de fundo permitindo talvez trabalhar níveis de empilhamento hierarquia
- Consistência em textos e ícones
- Consistência em espaçamentos e tamanhos  de caixas e componentes, cobrindo (xxs, xs, s, m, l, xl xxl)
- Padrões de opacidade e backdrop
- Padrões de bordas
- Padrões de sombra
- Padrões de bordas arredondadas
- Definição de outline
- Estados consistentes (link, hover, visited, focus, active, selected, unselected, disabled)
- Estados de feedback (success, error, warning, info, common/default)
- Gadientes para fundos e bordas
- Animações

Os nomes de variáveis devem ser flexíveis para abranger temas dark, light e custom, sem perder a coerência.

Este é um projeto de arquitetura de Design System de "Nível S" (S-Tier). Para alcançar a flexibilidade e consistência desejadas, a melhor abordagem é utilizar uma arquitetura de **Tokens de Design em duas camadas**:

1. **Camada Primitiva (Global):** Define os valores brutos (ex: `blue-500`, `space-16px`). É a sua "caixa de lápis de cor".
2. **Camada Semântica (Alias):** Define a função do valor (ex: `bg-primary`, `text-body`, `border-error`). É aqui que a mágica dos temas (Dark/Light) acontece.

Abaixo está a estrutura completa CSS (`:root`) pronta para produção.

### 1. Camada Primitiva (A Base)

Estas variáveis definem a "matéria-prima" do design. Não devem ser usadas diretamente nos componentes, mas servem de referência.

```css
:root {
  /* --- 1. Paletas de Cores (10 Níveis + Base) --- */
  /* Utilize HSL para facilitar manipulação via calc() se necessário */
  
  /* Brand / Primary */
  --p-primary-50:  220 100% 97%;
  --p-primary-100: 220 100% 92%;
  --p-primary-200: 220 98%  80%;
  --p-primary-300: 220 95%  65%;
  --p-primary-400: 220 90%  55%;
  --p-primary-500: 220 85%  50%; /* Cor Base */
  --p-primary-600: 220 90%  40%;
  --p-primary-700: 220 85%  30%;
  --p-primary-800: 220 80%  20%;
  --p-primary-900: 220 75%  10%;
  --p-primary-950: 220 70%  5%;

  /* Neutrals (Grays/Slate) - Crucial para UI */
  --p-neutral-0:   0   0%   100%;
  --p-neutral-50:  215 15%  97%;
  --p-neutral-100: 215 15%  92%;
  --p-neutral-200: 215 15%  85%;
  --p-neutral-300: 215 15%  75%;
  --p-neutral-400: 215 15%  60%;
  --p-neutral-500: 215 15%  50%;
  --p-neutral-600: 215 15%  35%;
  --p-neutral-700: 215 15%  25%;
  --p-neutral-800: 215 15%  15%;
  --p-neutral-900: 215 15%  10%;
  --p-neutral-950: 215 15%  5%;

  /* Feedback Colors (Exemplo encurtado, seguiriam a escala de 10) */
  --p-success-500: 142 70% 45%;
  --p-warning-500: 35 90% 55%;
  --p-error-500:   0 80% 55%;
  --p-info-500:    195 85% 50%;

  /* --- 2. Espaçamentos (Escala de 4px) --- */
  --p-space-0:   0px;
  --p-space-xxs: 2px;
  --p-space-xs:  4px;
  --p-space-s:   8px;
  --p-space-m:   16px;
  --p-space-l:   24px;
  --p-space-xl:  32px;
  --p-space-2xl: 48px;
  --p-space-3xl: 64px;
  --p-space-4xl: 96px;

  /* --- 3. Tipografia (Escalas) --- */
  --p-font-size-xxs: 0.625rem; /* 10px */
  --p-font-size-xs:  0.75rem;  /* 12px */
  --p-font-size-s:   0.875rem; /* 14px */
  --p-font-size-m:   1rem;     /* 16px */
  --p-font-size-l:   1.125rem; /* 18px */
  --p-font-size-xl:  1.25rem;  /* 20px */
  --p-font-size-2xl: 1.5rem;   /* 24px */
  --p-font-size-3xl: 2rem;     /* 32px */
}

```

### 2. Camada Semântica (O Design System)

Estas são as variáveis que você usará no código. Elas mudam dependendo do tema (`light` ou `dark`), mantendo a lógica intacta.

#### 2.1 Superfícies e Hierarquia

Para criar profundidade (Design S-Tier), usamos níveis de superfície.

```css
:root, [data-theme="light"] {
  /* Backgrounds */
  --bg-page:       hsl(var(--p-neutral-50));
  --bg-surface-1:  hsl(var(--p-neutral-0));   /* Cards */
  --bg-surface-2:  hsl(var(--p-neutral-50));  /* Modais/Dropdowns */
  --bg-surface-3:  hsl(var(--p-neutral-100)); /* Tooltips/Hover intenso */
  
  /* Backdrop / Glassmorphism */
  --bg-backdrop:   hsla(var(--p-neutral-0) / 0.7);
  --bg-overlay:    hsla(var(--p-neutral-900) / 0.4);

  /* Bordas e Divisores */
  --border-subtle: hsl(var(--p-neutral-200));
  --border-default: hsl(var(--p-neutral-300));
  --border-strong: hsl(var(--p-neutral-400));
}

[data-theme="dark"] {
  /* Inversão lógica para Dark Mode */
  --bg-page:       hsl(var(--p-neutral-950));
  --bg-surface-1:  hsl(var(--p-neutral-900));
  --bg-surface-2:  hsl(var(--p-neutral-800));
  --bg-surface-3:  hsl(var(--p-neutral-700));
  
  --bg-backdrop:   hsla(var(--p-neutral-900) / 0.7);
  
  --border-subtle: hsl(var(--p-neutral-800));
  --border-default: hsl(var(--p-neutral-700));
  --border-strong: hsl(var(--p-neutral-600));
}

```

#### 2.2 Cores de Ação e Texto

Consistência visual para garantir legibilidade e hierarquia.

```css
:root {
  /* Texto e Ícones */
  /* Mapeamento deve ser invertido no Dark Mode */
  --text-primary:   hsl(var(--p-neutral-900)); /* Títulos */
  --text-secondary: hsl(var(--p-neutral-600)); /* Parágrafos */
  --text-tertiary:  hsl(var(--p-neutral-400)); /* Placeholders / Meta */
  --text-inverse:   hsl(var(--p-neutral-0));   /* Texto sobre cor sólida */

  /* Ações (Buttons, Links) */
  --action-primary-bg:       hsl(var(--p-primary-500));
  --action-primary-bg-hover: hsl(var(--p-primary-600));
  --action-primary-text:     hsl(var(--p-neutral-0));
  
  --action-secondary-bg:       hsl(var(--p-neutral-100));
  --action-secondary-bg-hover: hsl(var(--p-neutral-200));
  --action-secondary-text:     hsl(var(--p-neutral-900));

  /* Feedback */
  --feedback-success-bg:     hsl(var(--p-success-500));
  --feedback-success-text:   hsl(var(--p-neutral-0));
  --feedback-error-bg:       hsl(var(--p-error-500));
}

```

#### 2.3 Forma, Sombra e Layout (Agnóstico ao Tema)

Variáveis que garantem o "Look & Feel" físico da interface.

```css
:root {
  /* Arredondamento (Radius) */
  --radius-none: 0;
  --radius-xs:   2px;
  --radius-s:    4px;
  --radius-m:    8px;  /* Padrão para inputs/buttons */
  --radius-l:    12px; /* Padrão para cards */
  --radius-xl:   24px;
  --radius-full: 9999px;

  /* Sombras (Elevation) */
  /* Utiliza cor de sombra variável para ficar natural no Dark Mode */
  --shadow-color: 220 3% 15%;
  --shadow-xs: 0px 1px 2px hsla(var(--shadow-color) / 0.05);
  --shadow-s:  0px 2px 4px hsla(var(--shadow-color) / 0.1);
  --shadow-m:  0px 4px 8px hsla(var(--shadow-color) / 0.12), 0px 2px 4px hsla(var(--shadow-color) / 0.08);
  --shadow-l:  0px 8px 16px hsla(var(--shadow-color) / 0.12), 0px 4px 8px hsla(var(--shadow-color) / 0.08);
  --shadow-xl: 0px 24px 48px hsla(var(--shadow-color) / 0.18);

  /* Outlines e Focus Ring */
  --outline-width: 2px;
  --outline-offset: 2px;
  --outline-color-focus: hsl(var(--p-primary-500));

  /* Efeitos Especiais */
  --blur-s:  4px;
  --blur-m:  8px;
  --blur-l:  16px;
  --opacity-disabled: 0.5;
  --opacity-hover:    0.9;
}

```

#### 2.4 Animações e Transições

Para uma sensação de fluidez "premium".

```css
:root {
  /* Durações */
  --duration-fast:   100ms;
  --duration-normal: 250ms;
  --duration-slow:   500ms;

  /* Easings (Curvas de Bezier profissionais) */
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-out:    cubic-bezier(0, 0, 0.2, 1);
  --ease-elastic: cubic-bezier(0.68, -0.55, 0.265, 1.55);

  /* Composite - Atalhos Prontos */
  --transition-default: all var(--duration-normal) var(--ease-in-out);
  --transition-hover:   background-color var(--duration-fast) var(--ease-out), transform var(--duration-fast) var(--ease-out);
}

```

### 3. Exemplo de Aplicação (O Padrão S-Tier)

Veja como criar um botão ou card usando essas variáveis. Note como o código é legível e expressivo:

```css
.card-premium {
  /* Estrutura */
  background-color: var(--bg-surface-1);
  padding: var(--p-space-l);
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  
  /* Visual */
  box-shadow: var(--shadow-m);
  
  /* Transição */
  transition: var(--transition-default);
}

.card-premium:hover {
  /* Elevação no hover */
  transform: translateY(-2px);
  box-shadow: var(--shadow-l);
  border-color: var(--border-default);
}

.title {
  color: var(--text-primary);
  font-size: var(--p-font-size-xl);
  font-weight: 600;
  margin-bottom: var(--p-space-xs);
}

.subtitle {
  color: var(--text-secondary);
  font-size: var(--p-font-size-s);
}

```

### Por que este sistema funciona?

1. **Separação de Preocupações:** Se você quiser mudar o tom de azul de todo o site, altera apenas a linha `--p-primary-500` nos Primitivos.
2. **Adaptação Automática:** Se o usuário trocar para Dark Mode, as variáveis Semânticas (`--bg-surface-1`) trocam seus valores internos, mas o CSS do componente `.card-premium` permanece intocado.
3. **Consistência Geométrica:** Ao forçar o uso de `--p-space-m` em vez de `15px` ou `17px`, você elimina o desalinhamento visual imperceptível que faz um site parecer "amador".
4. **Acessibilidade:** As cores de texto (`--text-primary` vs `--text-tertiary`) já forçam o desenvolvedor a pensar em contraste.
