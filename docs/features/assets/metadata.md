# Metadados e organização

Metadados são informações que ajudam a descrever um arquivo. O Mundam reúne
dados que já existem no sistema de arquivos e informações que consegue
interpretar a partir do conteúdo, quando o formato oferece suporte.

## Informações do arquivo

Para cada asset indexado, o catálogo do Mundam registra:

- nome e formato;
- tamanho do arquivo;
- localização na biblioteca;
- datas de criação e modificação fornecidas pelo sistema de arquivos;
- data em que o asset foi adicionado ou atualizado no catálogo.

O extrator do formato pode fornecer dados técnicos adicionais. Dependendo do
tipo de arquivo, isso pode incluir largura e altura, duração ou outros campos
específicos. Em imagens compatíveis, o Inspector também pode apresentar dados
EXIF. A disponibilidade e os nomes dos campos variam de acordo com o arquivo:
um campo ausente não significa necessariamente que o arquivo esteja com
problema.

Os dados técnicos são obtidos por meio do suporte disponível para cada formato.
O Mundam não promete que todos os formatos forneçam todos os campos, nem que
metadados internos sejam sempre exibidos na mesma seção.

## Tags, favoritos, avaliações e anotações

Além dos dados extraídos, você pode organizar os assets com informações suas:

- **Tags**: crie etiquetas e associe uma ou várias a um asset. Ao selecionar
  vários assets, você pode adicionar ou remover tags em conjunto; o Inspector
  destaca tags que estão presentes apenas em parte da seleção.
- **Favoritos**: marque assets que quer encontrar com facilidade.
- **Avaliações**: atribua uma nota de zero a cinco estrelas.
- **Anotações**: escreva observações pessoais para acompanhar o asset.

Essas informações de organização são guardadas pelo Mundam no catálogo da
biblioteca. Elas não são gravadas de volta no arquivo original.

## Como as informações ajudam na busca

Os dados indexados também servem para filtrar a biblioteca. A busca avançada
pode combinar critérios como nome do arquivo, formato, tamanho, datas de
criação/modificação/adicionamento, largura, altura, avaliação, pasta, tags,
anotações e cor. Os grupos de critérios podem usar condições **E** ou **OU**.

Quando existem dados de cores para um asset, a busca avançada também pode
procurar por uma cor e ajustar a tolerância de proximidade. Veja
[Cores, paletas e harmonia](./colors.md#buscar-assets-por-cor).

As pastas inteligentes guardam uma consulta para que você possa voltar a um
conjunto de resultados sem organizar os arquivos manualmente em uma nova pasta.

> **Sugestão de captura:** o Inspector de uma imagem com as seções de informação
> geral, detalhes da imagem, tags, dados avançados e paleta de cores abertas.

Saiba onde essas informações aparecem no [Inspector](./views.md#inspector) e
na [Viewport](./views.md#viewport).