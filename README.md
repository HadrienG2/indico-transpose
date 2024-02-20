# Indico registration data transpose

The registration form for the [I2I computing
course](https://indico.ijclab.in2p3.fr/event/10290/) produces a CSV with one row
per applicant, and a composite column that says which course(s) they applied to.

But as teachers, what we actually want to know is who registered to each course.

Hence this program, which ingests the indico CSV and tells who registered to
each course in Markdown format.
