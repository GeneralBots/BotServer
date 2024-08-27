REM Monthly consumption of bars (Individual sending to each customer)

customers = FIND "Customers"
FOR EACH c IN customers
    data = SELECT SUM(UnitPrice * Quantity) as Value, MONTH(OrderDate)+'/'+YEAR(OrderDate) from billing 
    JOIN Customers ON billing.CustomerID = Customers.CustomerID 
    GROUP BY MONTH(OrderDate), YEAR(OrderDate) 
    WHERE Customers.CustomerID = c.CustomerID
    img  = CHART "timseries", data
    SEND FILE img, "Monthly Consumption"
END FOR
