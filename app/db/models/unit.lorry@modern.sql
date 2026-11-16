-- unit.lorry@modern: Lorry (mesh.lorry_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lorry_modern','Lorry');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lorry_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels_front','wheel','mat.rubber',0,'','{"x":-0.5,"z":0.95,"w":0.58,"t":0.16,"spokes":1,"segments":10,"repeat":2,"dx":1}'),
  ('wheels_mid','wheel','mat.rubber',0,'','{"x":-0.5,"z":-0.32,"w":0.58,"t":0.18,"spokes":1,"segments":10,"repeat":2,"dx":1}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.5,"z":-0.94,"w":0.58,"t":0.18,"spokes":1,"segments":10,"repeat":2,"dx":1}'),
  ('chassis','box','mat.cast_iron',0,'','{"y":0.22,"z":0.05,"w":0.7,"d":2.6,"h":0.12,"far":0}'),
  ('bonnet','box','mat.olive_drab',1,'','{"y":0.34,"z":1.08,"w":0.62,"d":0.5,"h":0.36,"taper":0.06}'),
  ('grille','box','mat.cast_iron',0,'','{"y":0.34,"z":1.34,"w":0.56,"d":0.04,"h":0.34,"far":0}'),
  ('guards','box','mat.olive_drab',0,'','{"x":-0.5,"y":0.6,"z":0.95,"w":0.22,"d":0.6,"h":0.04,"repeat":2,"dx":1,"far":0}'),
  ('cab','box','mat.olive_drab',1,'','{"y":0.34,"z":0.6,"w":1,"d":0.48,"h":0.66,"windows":2,"window_y":0.4,"window_w":0.32,"window_h":0.18,"side_windows":1,"opening":"#22262a"}'),
  ('cab_roof','box','mat.olive_drab',0,'','{"y":1,"z":0.6,"w":1.02,"d":0.52,"h":0.04,"far":0}'),
  ('bed','box','mat.olive_drab',0,'','{"y":0.34,"z":-0.6,"w":1.08,"d":1.66,"h":0.2}'),
  ('tilt','box','mat.khaki',0,'','{"y":0.54,"z":-0.6,"w":1.06,"d":1.62,"h":0.56,"gable":10}'),
  ('tailboard','box','mat.olive_drab',1,'','{"y":0.34,"z":-1.44,"w":1.08,"d":0.04,"h":0.3}'),
  ('tilt_band','box','mat.cloth',1,'','{"y":0.56,"z":-0.6,"w":1.08,"d":1.64,"h":0.12,"far":0}'),
  ('star','box','mat.canvas',0,'cab','{"x":0.505,"y":0.3,"z":0,"w":0.01,"d":0.14,"h":0.14,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lorry_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.lorry'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.lorry_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
