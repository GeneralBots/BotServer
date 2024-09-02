PARAM product AS string LIKE telephone DESCRIPTION The name of the product to have the price retrieved.
DESCRIPTION Returns the price of the given product.

product = FIND "products.csv", "name = ${product}"
price = product.price
RETURN price 