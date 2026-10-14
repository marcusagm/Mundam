# Cores, paletas e harmonia

O Mundam pode analisar a miniatura de um asset e guardar uma paleta de cores
representativas. Assim, você consegue consultar as cores predominantes sem
precisar abrir cada arquivo.

## Como a paleta é criada

Depois que uma miniatura compatível é gerada, uma tarefa em segundo plano
analisa seus pixels e agrupa cores visualmente próximas. A análise considera
até 24 grupos de cor e guarda a cor em hexadecimal e a proporção aproximada
que ela representa na imagem. Pixels muito transparentes são ignorados.

Como a análise é feita a partir da miniatura, a paleta é uma representação
resumida, não uma leitura exata de cada pixel do arquivo original. Ela pode não
estar disponível se não houver uma miniatura adequada ou se a extração falhar.
Quando nenhum dado foi extraído, o Inspector informa que a paleta ainda não
está disponível.

## Paleta, distribuição e harmonia

Na seção **Color Palette** do [Inspector](./views.md#inspector), você pode
consultar:

- **Palette**: amostras das cores encontradas, ordenadas pelas que ocupam maior
  proporção. Clique em uma amostra para copiar seu código hexadecimal. O menu
  de contexto oferece a cópia em outros formatos de cor.
- **Distribution**: uma barra que mostra como as famílias de cores se dividem
  na imagem e a participação aproximada de cada grupo.
- **Harmony**: uma classificação calculada a partir dos grupos de cores, como
  complementar, análoga, triádica, monocromática, acromática ou neutra.

A harmonia é uma sugestão automática baseada nas cores identificadas, não uma
avaliação artística. Imagens com uma combinação que não se encaixa nos padrões
reconhecidos podem aparecer como **Not Identified**.

## Buscar assets por cor

Na busca avançada, escolha uma cor e ajuste a tolerância para procurar assets
com cores extraídas próximas à escolhida. Uma tolerância menor procura tons
mais parecidos; uma tolerância maior aceita uma gama mais ampla de cores.

Essa busca depende da paleta que já foi extraída e armazenada. Portanto, um
asset sem dados de cor ainda pode não aparecer nos resultados, mesmo que
visualmente contenha a cor escolhida.

> **Sugestão de captura:** a paleta de uma imagem no Inspector, incluindo a
> harmonia e a barra de distribuição. Uma segunda captura pode mostrar o
> critério de cor da busca avançada e seu controle de tolerância.

Veja também como o Mundam [gera miniaturas](./thumbnails.md) e como essas
informações aparecem no [Inspector](./views.md#inspector).