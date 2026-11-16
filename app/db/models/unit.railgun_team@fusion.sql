-- unit.railgun_team@fusion: Railgun team (mesh.railgun_team_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.railgun_team_fusion','Railgun team');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.railgun_team_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('loader','figure','mat.cloth',1,'','{"x":0.5,"z":-0.35,"yaw":-25,"attack":7,"hold_right":30,"hold_left":30,"legs":"#4d545c"}'),
  ('loader_helmet','dome','mat.white_composite',0,'loader.head','{"y":-0.12,"w":0.3,"h":0.17,"segments":6}'),
  ('loader_visor','box','mat.light_strip',0,'loader.head','{"y":-0.07,"z":0.115,"w":0.2,"d":0.04,"h":0.045,"far":0}'),
  ('loader_cell','cylinder','mat.graphite',0,'loader.back','{"y":-0.3,"z":-0.06,"w":0.2,"h":0.5,"segments":6}'),
  ('loader_glow','cylinder','mat.plasma',0,'loader_cell','{"y":0.16,"w":0.21,"h":0.14,"segments":6,"far":0}'),
  ('gunner','figure','mat.cloth',1,'','{"x":-0.12,"y":-0.1,"z":-0.78,"seat":1,"attack":7,"hold_right":70,"hold_left":70,"legs":"#4d545c"}'),
  ('gunner_helmet','dome','mat.white_composite',0,'gunner.head','{"y":-0.12,"w":0.3,"h":0.17,"segments":6,"far":0}'),
  ('gunner_visor','box','mat.light_strip',0,'gunner.head','{"y":-0.07,"z":0.115,"w":0.2,"d":0.04,"h":0.045,"far":0}'),
  ('tripod_front','box','mat.graphite',0,'','{"x":-0.12,"z":0.15,"w":0.035,"d":0.035,"h":0.44,"pitch":-28,"far":0}'),
  ('tripod_w','box','mat.graphite',0,'','{"x":-0.36,"z":-0.36,"w":0.035,"d":0.035,"h":0.46,"roll":28,"pitch":26,"far":0}'),
  ('tripod_e','box','mat.graphite',0,'','{"x":0.12,"z":-0.36,"w":0.035,"d":0.035,"h":0.46,"roll":-28,"pitch":26,"far":0}'),
  ('breech','box','mat.white_composite',1,'','{"x":-0.12,"y":0.42,"z":-0.5,"w":0.24,"d":0.4,"h":0.2}'),
  ('rails','box','mat.graphite',0,'','{"x":-0.12,"y":0.52,"z":-0.62,"w":0.14,"d":0.1,"h":1.9,"pitch":87}'),
  ('rail_glow','box','mat.plasma',0,'rails','{"y":0.4,"z":-0.055,"w":0.04,"d":0.02,"h":1.4,"far":0}'),
  ('muzzle','box','mat.white_composite',0,'rails','{"y":1.9,"w":0.2,"d":0.14,"h":0.08,"far":0}'),
  ('spark','cone','mat.plasma',0,'rails','{"y":1.98,"w":0.16,"h":0.16,"segments":3,"flash":1,"far":0}'),
  ('streak','cylinder','mat.light_strip',0,'spark','{"y":0.16,"w":0.05,"h":1.4,"segments":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.36,"w":0.4,"h":0.36,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.railgun_team_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.railgun_team'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.railgun_team_fusion'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
