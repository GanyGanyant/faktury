#import "@preview/tiaoma:0.3.0": qrcode

#let data = json("invoice_data.json")

#let fmt(n) = {
  let s = str(calc.round(n, digits: 2))
  let parts = s.split(".")
  let int_part = parts.at(0)
  let dec_part = if parts.len() > 1 {
    let d = parts.at(1)
    "," + if d.len() == 1 { d + "0" } else { d }
  } else { "" }
  let chars = int_part.clusters().rev()
  let grouped = ()
  for (i, c) in chars.enumerate() {
    if i > 0 and calc.rem(i, 3) == 0 and c != "-" {
      grouped.push("\u{00A0}")
    }
    grouped.push(c)
  }
  grouped.rev().join() + dec_part
}

#set page(paper: "a4", margin: (x: 1.8cm, top: 2.2cm, bottom: 1.8cm))
#set text(font: "Libertinus Sans", size: 9.5pt, fill: rgb("#1e293b"), lang: "cs")

// Top color accent bar across the top of page
#place(top + left, dx: -1.8cm, dy: -2.2cm,
  rect(width: 100% + 3.6cm, height: 7pt, fill: rgb("#2563eb"))
)

// Header: Title and QR Code
#grid(
  columns: (1fr, auto),
  align: (left, right),
  [
    #v(0.2cm)
    #text(size: 22pt, weight: "bold", fill: rgb("#0f172a"))[FAKTURA] \
    #v(-0.3em)
    #text(size: 11pt, fill: rgb("#64748b"))[Daňový doklad č.: *#data.invoice_number*]
  ],
  [
    #if data.supplier.iban != "" [
      #block(
        stroke: 0.5pt + rgb("#cbd5e1"),
        radius: 6pt,
        inset: 6pt,
        fill: rgb("#ffffff")
      )[
        #qrcode(
          "SPD*1.0*ACC:" + data.supplier.iban + "*AM:" + str(data.total_with_vat) + "*CC:CZK*X-VS:" + data.variable_symbol.replace("/", ""),
          options: (scale: 1.7)
        )
        #align(center)[#text(size: 7pt, weight: "bold", fill: rgb("#64748b"))[QR Platba]]
      ]
    ]
  ]
)

#v(0.8em)

// Supplier and Customer Cards
#grid(
  columns: (1fr, 1fr),
  gutter: 1.2cm,
  [
    #block(
      width: 100%,
      inset: 10pt,
      radius: 6pt,
      fill: rgb("#f8fafc"),
      stroke: 0.5pt + rgb("#e2e8f0")
    )[
      #text(size: 8pt, weight: "bold", fill: rgb("#2563eb"))[DODAVATEL] \
      #v(0.2em)
      #text(size: 11pt, weight: "bold", fill: rgb("#0f172a"))[#data.supplier.name] \
      #data.supplier.street \
      #data.supplier.zip #data.supplier.city \
      #v(0.4em)
      #grid(
        columns: (auto, 1fr),
        gutter: 6pt,
        [*IČO:*], [#data.supplier.ico],
        ..if data.supplier.dic != "" { ([*DIČ:*], [#data.supplier.dic]) } else { () },
        [*Účet:*], [#data.supplier.bank_account],
        [*IBAN:*], [#data.supplier.iban]
      )
    ]
  ],
  [
    #block(
      width: 100%,
      inset: 10pt,
      radius: 6pt,
      fill: rgb("#f8fafc"),
      stroke: 0.5pt + rgb("#e2e8f0")
    )[
      #text(size: 8pt, weight: "bold", fill: rgb("#0284c7"))[ODBĚRATEL] \
      #v(0.2em)
      #text(size: 11pt, weight: "bold", fill: rgb("#0f172a"))[#data.customer.name] \
      #data.customer.street \
      #data.customer.zip #data.customer.city \
      #v(0.4em)
      #grid(
        columns: (auto, 1fr),
        gutter: 6pt,
        [*IČO:*], [#data.customer.ico],
        ..if data.customer.dic != "" { ([*DIČ:*], [#data.customer.dic]) } else { () }
      )
    ]
  ]
)

#v(0.6em)

// Invoice parameters
#block(
  fill: rgb("#eff6ff"),
  inset: 10pt,
  radius: 6pt,
  stroke: 0.5pt + rgb("#bfdbfe"),
  width: 100%
)[
  #grid(
    columns: (1fr, 1fr, 1fr, 1fr),
    gutter: 8pt,
    [#text(size: 8pt, fill: rgb("#64748b"))[Datum vystavení] \ *#data.issue_date*],
    [#text(size: 8pt, fill: rgb("#64748b"))[Datum splatnosti] \ *#data.due_date*],
    [#text(size: 8pt, fill: rgb("#64748b"))[Variabilní symbol] \ *#data.variable_symbol*],
    [#text(size: 8pt, fill: rgb("#64748b"))[Forma úhrady] \ *#data.payment_method*]
  )
]

#v(0.6em)

#if data.note != "" [
  #block(
    stroke: (left: 3pt + rgb("#2563eb")),
    fill: rgb("#f8fafc"),
    inset: (left: 10pt, rest: 8pt),
    radius: (right: 4pt),
    width: 100%
  )[
    #text(size: 8.5pt, weight: "bold", fill: rgb("#475569"))[Poznámka:] \
    #data.note
  ]
  #v(0.6em)
]

// Line items table
#table(
  columns: (1fr, auto, auto, auto, auto),
  align: (left, right, right, right, right),
  stroke: (x: none, y: 0.5pt + rgb("#e2e8f0")),
  inset: (x: 8pt, y: 7pt),
  fill: (_, y) => if y == 0 { rgb("#1e293b") } else if calc.even(y) { rgb("#f8fafc") },
  table.header(
    [#text(fill: white, weight: "bold")[Položka]],
    [#text(fill: white, weight: "bold")[Množství]],
    [#text(fill: white, weight: "bold")[Cena/ks]],
    [#text(fill: white, weight: "bold")[DPH]],
    [#text(fill: white, weight: "bold")[Celkem]]
  ),
  ..data.items.map(item => (
    [#item.description],
    [#item.quantity #item.unit],
    [#fmt(item.unit_price) Kč],
    [#item.vat_rate %],
    [*#fmt(item.total) Kč*],
  )).flatten()
)

#v(0.8em)

// Totals summary
#align(right)[
  #block(width: 52%)[
    #grid(
      columns: (1fr, auto),
      column-gutter: 14pt,
      row-gutter: 7pt,
      align: (left, right),
      [Celkem bez DPH:], [#fmt(data.total_without_vat) Kč],
      [DPH celkem:], [#fmt(data.total_vat) Kč],
      grid.hline(stroke: 1pt + rgb("#cbd5e1")),
      [#v(2pt) #text(size: 12pt, weight: "bold", fill: rgb("#0f172a"))[K úhradě:]],
      [#v(2pt) #text(size: 13pt, weight: "bold", fill: rgb("#2563eb"))[#fmt(data.total_with_vat) Kč]]
    )
  ]
]
