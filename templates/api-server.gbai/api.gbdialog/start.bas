REM 302 / 1234
PARAM barraca AS number LIKE Código da barraca
PARAM operador AS number LIKE Código do operador
DESCRIPTION Esta função (tool) nunca é chamada pelo GPT. É um WebService do GB.

REM Login como Garçom
data = NEW OBJECT
data.IdentificadorOperador = operador
data.BarracaId = barraca
login = POST "https://api.server.com.br/api/Operadores/Login", data
SET HEADER "Authorization" AS  login.accessToken

REM Obter o cardápio da Barraca -  Utilizar o token recuperado acima. 
data = GET "https://api.server.com.br/api/Item/Barraca/${barraca}/Cliente"
produtos = NEW ARRAY 
 
FOR EACH item IN data[0].itens
     IF item.statusItem = "Ativo" THEN
        produto = NEW OBJECT
        produto.id = item.id
        produto.valor = item.valor
        produto.nome = item.produto.nome
        produto.detalhe = item.detalhe
        produto.acompanhamentos = item.gruposAcompanhamento

        produtos.push(produto)
    END IF
NEXT

BEGIN SYSTEM PROMPT
Você deve atuar como um chatbot que irá auxiliar o atendente de uma loja respeitando as seguintes regras:
Sempre que o atendente fizer um pedido e deve incluir a mesa e o nome do cliente. Exemplo: Uma caipirinha de 400ml de Abacaxi para Rafael na mesa 10.
Os pedidos são feitos com base nos produtos e acompanhamentos deste cardápio de produtos:
 ${JSON.stringify (produtos)}.
A cada pedido realizado, retorne JSON contendo o nome do produto, a mesa e uma lista de acompanhamentos com seus respectivos ids.
Mantenha itensPedido com apenas um item e mantenha itemsAcompanhamento apenas com os acompanhamentos que foram especificados.
ItensAcompanhamento deve conter a coleção de itens de acompanhamento do pedido, que é solicitado quando o pedido é feito, por exemplo: Caipirinha de Morango com Gelo, Açúcar e Limão, gerariam três elementos neste nó.

Segue o exemplo do JSON do Pedido, apague os itens e mande um com o pedido feito pela pessoa, é apenas um exemplo: 
{
    itensPedido: [
        {
           item: {
                id: 23872,
                valor: 20,
                nome: Guaraná
            },
            itensAcompanhamento: [
                 {
                    id: 0,
                    valor: 0,
                    quantidade: 1
                }
            ],
            quantidade: 1,
            observacao: a
        },
        {
            item: {
                id: 25510,
                valor: 12,
                nome: Laranja Lata 350ml
            },
            itensAcompanhamento: [],
            quantidade: 1,
            observacao: nenhuma
        }
    ],
    barracaId: ${barraca},
    usuarioId: ${operador},
    identificadorConta: Areia,
    tipoEntregaId: 2,
    camposTipoEntrega: {
        Mesa: 5
    }
}


END SYSTEM PROMPT