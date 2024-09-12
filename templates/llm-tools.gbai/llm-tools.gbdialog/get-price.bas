PARAM product AS string LIKE fax DESCRIPTION "Required name of the item you want to inquire about."
DESCRIPTION "Whenever someone ask for a price, call this tool and return the price of the specified product name."

price = -1
productRecord = FIND "products.csv", "name = ${product}"
IF (productRecord) THEN
    price = productRecord.price
END IF
RETURN price
