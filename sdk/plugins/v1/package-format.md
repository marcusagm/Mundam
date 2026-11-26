# Formato de pacote declarativo v1

O arquivo `.mundam-plugin` v1 carrega um manifesto e seu ícone em posições fixas. O envelope armazena os bytes originais sem compressão e sem diretório interno. Cada posição recebe exatamente um recurso. O caminho lógico do ícone vem do manifesto verificado.

| Ordem | Campo                              | Codificação                                                              |
| ----- | ---------------------------------- | ------------------------------------------------------------------------ |
| 1     | Identificador e versão do envelope | Bytes ASCII de `MUNDAM-PLUGIN`, seguidos por `00 01` em hexadecimal      |
| 2     | Tamanho do manifesto               | Inteiro de 32 bits sem sinal em big-endian, entre 1 e 65.536             |
| 3     | Tamanho do ícone                   | Inteiro de 32 bits sem sinal em big-endian, entre 1 e 262.144            |
| 4     | Manifesto                          | Bytes do manifesto v1 em UTF-8                                           |
| 5     | Ícone                              | PNG estático conforme os limites do SDK                                  |
| 6     | Chave pública do editor            | 32 bytes, codificação Ed25519 da chave pública                           |
| 7     | Assinatura                         | 64 bytes, assinatura Ed25519 sobre a concatenação exata dos campos 1 a 6 |

O tamanho máximo do arquivo é 327.799 bytes. O verificador rejeita truncamento, divergência de tamanho, versão desconhecida e bytes depois da assinatura. A assinatura opera sobre os bytes originais, sem normalizar o JSON. A separação entre o versionamento do envelope e do manifesto permite revisar o transporte em uma versão futura. [Ed25519 no RFC 8032](https://www.rfc-editor.org/rfc/rfc8032)

A política de confiança fornece um namespace exato e o SHA-256 dos 32 bytes brutos da chave pública. Esse fingerprint é diferente do hash do arquivo da chave ou de sua representação DER. O verificador compara o fingerprint esperado, confere a assinatura e valida então o manifesto e o PNG. A correspondência exige que o `publisher` validado seja igual ao namespace externo. A compatibilidade de execução é determinada no host pela versão da API e pelo alvo efetivo.

O SHA-256 do pacote completo identifica o arquivo exato que foi verificado. O futuro instalador pode vincular a aprovação a esse digest e à política de confiança vigente. O arquivo transporta uma declaração de permissões. Concessão, ativação, troca de chave e revogação pertencem à política do host.
