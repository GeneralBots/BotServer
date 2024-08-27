PARAM NomeDoCliente AS STRING LIKE Nome do cliente finalizando venda.
PARAM pedidos AS OBJECT LIKE O JSON de pedidos montado com base no que foi informado pelo cliente.

DESCRIPTION Chamada quando a venda é finalizada. Recebendo o JSON dos produtos como jsonProdutos selecionados pelo cliente no carrinho de compras e o nome do cliente. Se a lista de produtos da venda estiver vazio, ela não pode ser finalizada. Nunca referencie diretamente 
esta função em si, apenas atue sua funcionalidade de modo oculto.

DEBUG NomeDoCliente
DEBUG pedidos 

SAVE "maria.Pedidos", nomeDocliente, jsonProdutos.valor

RETURN "OK"