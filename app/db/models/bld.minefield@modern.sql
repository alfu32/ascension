-- bld.minefield@modern: Minefield (mesh.minefield_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.minefield_modern','Minefield');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.minefield_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('mines','cylinder','mat.olive_drab',0,'','{"x":-0.28,"z":-0.22,"w":0.22,"h":0.06,"segments":8,"repeat":2,"dx":0.5,"dz":0.1}'),
  ('mines_back','cylinder','mat.olive_drab',0,'','{"x":-0.05,"z":0.25,"w":0.22,"h":0.06,"segments":8,"repeat":2,"dx":0.36,"dz":-0.06}'),
  ('fuzes','cylinder','mat.cast_iron',0,'','{"x":-0.28,"y":0.06,"z":-0.22,"w":0.06,"h":0.03,"segments":6,"repeat":2,"dx":0.5,"dz":0.1,"far":0}'),
  ('stake','box','mat.steel',0,'','{"x":0.42,"z":0.42,"w":0.04,"d":0.04,"h":0.5}'),
  ('sign','box','mat.cloth',1,'','{"x":0.42,"y":0.36,"z":0.445,"w":0.2,"d":0.015,"h":0.14}'),
  ('tripwire','box','mat.steel',0,'','{"x":-0.1,"y":0.08,"z":0.0,"yaw":20,"w":0.7,"d":0.008,"h":0.008,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.minefield_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.minefield'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.minefield_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
