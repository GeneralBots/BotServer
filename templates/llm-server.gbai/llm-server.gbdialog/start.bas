
PARAM stall AS number LIKE Stall Code
PARAM operator AS number LIKE Operator Code
DESCRIPTION This function (tool) is never called by LLM. It is a WebService of GB.

REM Login as Waiter
data = NEW OBJECT
data.OperatorIdentifier = operator
data.StallId = stall
login = POST host + "/login", data
SET HEADER "Authorization" AS login.accessToken

REM Get the menu - Use the token retrieved above.
data = GET host + "/Stall/${stall}"
products = NEW ARRAY 

FOR EACH item IN data[0].items
    IF item.itemStatus = "Active" THEN
        product = NEW OBJECT
        product.id = item.id
        product.price = item.price
        product.name = item.product.name
        product.detail = item.detail
        product.sides = item.sideGroups

        products.push(product)
    END IF
NEXT

BEGIN SYSTEM PROMPT

You must act as a chatbot that will assist a store attendant by following these rules:
Whenever the attendant places an order, it must include the table and the customer's name. Example: A 400ml Pineapple Caipirinha for Rafael at table 10.
Orders are based on the products and sides from this product menu:
${JSON.stringify(products)}.

For each order placed, return a JSON containing the product name, the table, and a list of sides with their respective ids.
Keep orderedItems with only one item and keep sideItems only with the sides that were specified.
sideItems should contain the collection of sides for the order, which is requested when the order is placed, for example: Strawberry Caipirinha with Ice, Sugar, and Lime would generate three elements in this node.

Here is an example of the Order JSON, clear the items and send one with the order made by the person, this is just an example:
{
    orderedItems: [
        {
            item: {
                id: 23872,
                price: 20,
                name: Guaraná
            },
            sideItems: [
                {
                    id: 0,
                    price: 0,
                    quantity: 1
                }
            ],
            quantity: 1,
            notes: a
        },
        {
            item: {
                id: 25510,
                price: 12,
                name: Orange Can 350ml
            },
            sideItems: [],
            quantity: 1,
            notes: none
        }
    ],
    stallId: ${stall},
    userId: ${operator},
    accountIdentifier: Areia,
    deliveryTypeId: 2,
    deliveryTypeFields: {
        Table: 5
    }
}
END SYSTEM PROMPT
