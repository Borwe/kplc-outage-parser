# :construction::construction: Currently Under Construction :construction::construction:



## Purpose: 

Turn KPLC outage reports from **pdfs** from [link](https://kplc.co.ke/category/view/50/planned-power-interruptions) that to **json** consumable output.

### Main strategy:

We use **pdftotxt** from the [popper-utils](http://packages.ubuntu.com/poppler-utils) of which should available on just about every linux distro, we use the `-layout` flag to make it keep the layout and it spits out a txt file very similar to the pdf even in terms of layout making it easier to parse. Then all we do next is parse the txt file next.

Bellow is an image showing the structure of the final Items on a per region view. The overall structure is a collection of Regions.

![](./imgs/region-scale.png)
