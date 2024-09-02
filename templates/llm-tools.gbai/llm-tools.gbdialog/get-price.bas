PARAM product AS string LIKE "Product A"
DESCRIPTION "Returns the price of the given product."

product = FIND "products.csv", "name LIKE ${product}"
price = product.price
RETURN price
