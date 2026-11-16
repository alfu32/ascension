-- unit.grav_skimmer@starfaring: Grav Skimmer (mesh.grav_skimmer_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.grav_skimmer_starfaring','Grav Skimmer');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.grav_skimmer_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bike','box','mat.white_composite',0,'','{"y":0.42,"z":-0.05,"w":0.42,"d":1.4,"h":0.22,"taper":0.2,"bank":12}'),
  ('nose','box','mat.white_composite',1,'bike','{"y":0.02,"z":0.82,"w":0.34,"d":0.42,"h":0.17,"taper":0.55}'),
  ('fairing','box','mat.smart_glass',0,'bike','{"y":0.22,"z":0.42,"w":0.3,"d":0.16,"h":0.16,"taper":0.3,"pitch":-25}'),
  ('seat','box','mat.graphite',0,'bike','{"y":0.22,"z":0,"w":0.26,"d":0.5,"h":0.06}'),
  ('tail','box','mat.white_composite',0,'bike','{"y":0.12,"z":-0.72,"w":0.62,"d":0.28,"h":0.05,"taper":0.3,"far":0}'),
  ('lift_front','wheel','mat.glow',0,'bike','{"y":-0.1,"z":0.45,"roll":90,"w":0.36,"t":0.05,"spokes":0,"segments":8}'),
  ('lift_back','wheel','mat.glow',0,'bike','{"y":-0.1,"z":-0.45,"roll":90,"w":0.4,"t":0.05,"spokes":0,"segments":8,"far":0}'),
  ('vanes','box','mat.graphite',0,'bike','{"x":-0.24,"y":-0.06,"z":0,"w":0.04,"d":1.1,"h":0.14,"repeat":2,"dx":0.48,"far":0}'),
  ('drive','cylinder','mat.graphite',0,'bike','{"y":0.08,"z":-0.95,"w":0.2,"h":0.25,"segments":6,"pitch":90,"far":0}'),
  ('drive_glow','wheel','mat.glow',0,'bike','{"y":0.08,"z":-0.97,"yaw":90,"w":0.16,"t":0.03,"spokes":0,"segments":6,"far":0}'),
  ('guns','cylinder','mat.graphite',0,'bike','{"x":-0.16,"y":0,"z":0.7,"w":0.05,"h":0.45,"segments":4,"pitch":90,"repeat":2,"dx":0.32,"far":0}'),
  ('spark','box','mat.flash',0,'bike','{"y":0,"z":1.17,"w":0.44,"d":0.08,"h":0.08,"flash":2,"far":0}'),
  ('bolts','cylinder','mat.glow',0,'spark','{"x":-0.16,"y":0,"z":0.06,"w":0.05,"h":0.5,"segments":3,"pitch":90,"repeat":2,"dx":0.32,"far":0}'),
  ('rider','figure','mat.cloth',1,'bike','{"y":0,"z":0.12,"seat":1,"attack":7,"hold_right":62,"hold_left":62,"legs":"#4d545c","skin":"#c9ced2"}'),
  ('helmet','dome','mat.white_composite',0,'rider.head','{"y":-0.14,"w":0.3,"h":0.28,"segments":8,"far":0}'),
  ('visor','box','mat.light_strip',0,'rider.head','{"y":-0.08,"z":0.11,"w":0.22,"d":0.04,"h":0.06,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.grav_skimmer_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.grav_skimmer'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.grav_skimmer_starfaring'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
